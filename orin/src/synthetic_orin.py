"""The synthetic Orin: the robot computer's stand-in in the simulated loop.

The policy has to learn from what the robot will see and remember, so this
process does what the real Orin does with each camera frame - detect the
four markers, keep the last six detections as the policy's timestamped
window (synthetic-orin-plan.md). Its camera is the Unity player: it serves
the frame socket Unity connects to, triggers each frame there with the
pose the orchestrator sends, and gets the JPEG back (synthetic-loop-dia.md,
4-14). It writes each frame to <out>/frame_NNNNNN.jpg and one observation
line per frame to <out>/observations.jsonl, and replies to each pose with
the pool (14).

The start (synthetic-loop-dia.md): pre-warm the pool, open the frame
socket (the note at 4); connect to the orchestrator's control socket (5);
wait for Unity (7); tell the orchestrator {"type": "registered"} (8). The
end: the orchestrator closes the control socket (22), this process closes
the frame socket (23), Unity quits (24), and this process exits (25).

Messages, control socket (orchestrator):
- out text   {"type": "registered"}                  once, when Unity has
                                                     connected (8)
- in  text   {"id": n, "time": t, "joints": [...]}   the capture state (11)
- out text   {"id": n, "pool": [[age_ms, x0, y0, ... x3, y3] x6],
              "failed": bool}                        exactly one per pose,
                                                     in order (14): the
                                                     pool newest first;
                                                     failed ends the run
Messages, frame socket (Unity):
- out text   {"id": n, "joints": [...]}              the trigger (12): the
                                                     pose, forwarded as is
- in  binary the JPEG                                one per trigger (13)

Rules:
- Times are simulated time (O2): `time` is the capture time the
  orchestrator sends (MuJoCo's data.time), never a wall clock.
- The window holds the last 6 marker frames (capture time + 4 slots); the
  7th append drops the oldest. Ages = capture time - newest capture time
  (newest 0, older negative).
- Pre-warm (synthetic-loop-dia.md, the note at 4; The initial pool): at
  launch, before any frame, the window is six entries of no detection,
  (-1, -1) x4, at times -d1, -(d1+d2), ..., -(d1+...+d6) before the first
  capture (t = 0). Each gap dk is a fresh RND(MIN, MAX) ms draw,
  random.Random(--seed).uniform(--rnd-delay-min, --rnd-delay-max), d1
  first, so the same seed and band give the same pool. The band is the
  trainer's settings.toml rnd_delay_min and rnd_delay_max, passed on by
  the orchestrator (4). Every frame, the first included, is then one append:
  frame 0's ages are 0, -d1, ..., -(d1+...+d5).
- A failed frame (synthetic-loop-dia.md, the note at 13; O3): Unity's
  JPEG does not decode or is not 728x544. Its entry is (-1, -1) x4, the
  error is logged, and it does not fail the run on its own; when 3 of the
  pool's 6 entries are failed frames, the reply says failed and the run
  fails. Pre-warm entries are no detection, not failed frames.
- --frames-listen is the frame socket's host:port; the orchestrator tells
  Unity the same address (6).

observations.jsonl, one line per frame:
  {"id", "time", "slots": [[x, y] x4], "confidences": [c x4], "detect_ms",
   "pool": [[age_ms, x0, y0, x1, y1, x2, y2, x3, y3] x6], "failed"}
plus "error" for a failed frame, whose confidences and detect_ms are null.
`pool` and `failed` are exactly what the reply carries; `confidences` and
`detect_ms` are for checking, not part of the marker frame.

usage (from the repo root):
  uv run --project orin python orin/src/synthetic_orin.py
      --orchestrator ws://... --frames-listen 127.0.0.1:<port>
      --out <dir> --seed <seed> --rnd-delay-min <ms> --rnd-delay-max <ms>
      --engine <file> [--config <settings.toml>]
"""

import argparse
import collections
import json
import math
import random
import time
from dataclasses import dataclass
from pathlib import Path
from typing import TextIO

from websockets.sync.client import ClientConnection, connect

from detector import Detector, FrameError
from detector_config import DEFAULT_PATH, DetectorConfig
from frame_socket import FrameSocket

# The loop's frame contract (synthetic-loop-dia.md, Contracts).
FRAME_SIZE = (728, 544)

# The policy input's window length (synthetic-loop-dia.md, Contracts).
WINDOW = 6

# A slot with no detection (synthetic-loop-dia.md, Contracts).
NOT_DETECTED = (-1.0, -1.0)

# Failed frames among the pool's entries that fail the run
# (synthetic-loop-dia.md, the note at 13).
FAILED_FRAMES_FAIL_RUN = 3


@dataclass(frozen=True)
class Entry:
    time: float  # capture time, simulated seconds
    slots: list[tuple[float, float]]
    failed: bool = False  # Unity's frame broke the JPEG contract


def pre_warm(seed: int, gap_ms: tuple[float, float]) -> collections.deque[Entry]:
    """The pool before any frame: six no-detection entries before t = 0,
    each gap a draw from the latency band gap_ms = (min, max)."""
    draws = random.Random(seed)
    times = []
    t = 0.0
    for _ in range(WINDOW):
        t -= draws.uniform(*gap_ms) / 1000  # d1 first: the gap nearest t = 0
        times.append(t)
    window: collections.deque[Entry] = collections.deque(maxlen=WINDOW)
    for t in reversed(times):  # the window runs oldest first
        window.append(Entry(t, [NOT_DETECTED] * 4))
    return window


def listen_address(text: str) -> tuple[str, int]:
    """host:port, as --frames-listen takes it."""
    host, separator, port = text.rpartition(":")
    if not separator or not host or not port.isdigit() or not 1 <= int(port) <= 65535:
        raise argparse.ArgumentTypeError(f"expected host:port with a port 1-65535, got {text!r}")
    return host, int(port)


def pool_rows(window: collections.deque[Entry]) -> list[list[float]]:
    """The pool as the reply carries it: newest first, ages in ms from
    the newest capture (0, then negative), then x0, y0, ... x3, y3."""
    newest = window[-1].time
    return [
        [(entry.time - newest) * 1000, *(v for slot in entry.slots for v in slot)]
        for entry in reversed(window)
    ]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--orchestrator", required=True, help="the orchestrator's WebSocket URL")
    parser.add_argument(
        "--frames-listen", type=listen_address, required=True, help="host:port of the frame socket Unity connects to"
    )
    parser.add_argument("--out", type=Path, required=True, help="the run's output folder")
    parser.add_argument("--seed", type=int, required=True, help="the run's seed: the pre-warm gaps")
    parser.add_argument(
        "--rnd-delay-min", type=float, required=True, help="the latency band's lower bound, ms: the pre-warm gaps"
    )
    parser.add_argument("--rnd-delay-max", type=float, required=True, help="the latency band's upper bound, ms")
    parser.add_argument(
        "--config", type=Path, default=DEFAULT_PATH, help="the detector's settings (default: orin/settings.toml)"
    )
    parser.add_argument("--engine", type=Path, required=True, help="TensorRT engine built by build_engine.py")
    args = parser.parse_args()
    if args.seed < 0:
        parser.error(f"--seed is a whole number >= 0, got {args.seed}")
    band = (args.rnd_delay_min, args.rnd_delay_max)
    if not (math.isfinite(band[0]) and math.isfinite(band[1]) and 0 <= band[0] <= band[1]):
        parser.error(f"the latency band is 0 <= --rnd-delay-min <= --rnd-delay-max ms, got {band}")

    detector = Detector(DetectorConfig.load(args.config), args.engine, FRAME_SIZE)
    window = pre_warm(args.seed, band)
    # Before frame 0, the entries' times are their ages from t = 0.
    ages_ms = ", ".join(f"{entry.time * 1000:.3f}" for entry in reversed(window))
    print(f"synthetic orin: pre-warmed from seed {args.seed}, band {band} ms, ages {ages_ms} ms", flush=True)
    host, port = args.frames_listen
    frames = FrameSocket(host, port)
    print(f"synthetic orin: frame socket listening on ws://{host}:{port}", flush=True)
    log_path = args.out / "observations.jsonl"
    try:
        # Line-buffered: a run that fails midway leaves every finished line.
        with open(log_path, "w", buffering=1) as log, connect(args.orchestrator, max_size=None) as ws:
            print(f"synthetic orin: connected to {args.orchestrator}, logging {log_path}", flush=True)
            camera = frames.wait_for_camera()
            print("synthetic orin: Unity connected to the frame socket", flush=True)
            ws.send(json.dumps({"type": "registered"}))
            for message in ws:
                if isinstance(message, bytes):
                    raise RuntimeError("binary message on the control socket, expected a pose")
                pose = json.loads(message)
                frame_id, capture = int(pose["id"]), float(pose["time"])
                camera.send(json.dumps({"id": frame_id, "joints": pose["joints"]}))
                jpeg = camera.recv()
                if not isinstance(jpeg, bytes):
                    raise RuntimeError(f"frame {frame_id}: Unity sent text, expected the JPEG")
                (args.out / f"frame_{frame_id:06d}.jpg").write_bytes(jpeg)
                observe(detector, window, log, ws, frame_id, capture, jpeg)
    finally:
        frames.close()
        detector.close()


def observe(
    detector: Detector,
    window: collections.deque[Entry],
    log: TextIO,
    ws: ClientConnection,
    frame_id: int,
    capture: float,
    jpeg: bytes,
) -> None:
    """Detects, appends to the window, logs the line, and replies (14)."""
    started = time.perf_counter()
    observation: dict = {"id": frame_id, "time": capture}
    try:
        marker_frame = detector.detect(jpeg)
    except FrameError as error:
        # A failed frame: no detection, the error logged (the note at 13).
        window.append(Entry(capture, [NOT_DETECTED] * 4, failed=True))
        observation |= {
            "slots": [list(NOT_DETECTED)] * 4,
            "confidences": None,
            "detect_ms": None,
            "error": str(error),
        }
        print(f"synthetic orin: frame {frame_id} failed: {error}", flush=True)
    else:
        detect_ms = (time.perf_counter() - started) * 1000
        window.append(Entry(capture, marker_frame.slots))
        observation |= {
            "slots": [list(slot) for slot in marker_frame.slots],
            "confidences": marker_frame.confidences,
            "detect_ms": round(detect_ms, 3),
        }

    pool = pool_rows(window)
    failed = sum(entry.failed for entry in window) >= FAILED_FRAMES_FAIL_RUN
    observation |= {"pool": pool, "failed": failed}
    log.write(json.dumps(observation) + "\n")
    ws.send(json.dumps({"id": frame_id, "pool": pool, "failed": failed}))


if __name__ == "__main__":
    main()
