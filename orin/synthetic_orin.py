"""The synthetic Orin: the robot computer's stand-in in the simulated loop.

The policy has to learn from what the robot will see and remember, so this
process does what the real Orin does with each camera frame - detect the
four markers, keep the last six detections as the policy's timestamped
window - but takes its frames from the orchestrator instead of the camera
(synthetic-orin-plan.md). It connects to the orchestrator's WebSocket as a client,
like the Unity player, and writes one observation line per frame to
<out>/observations.jsonl. The policy slot is still empty: the reply is an
acknowledgement.

Messages (synthetic-orin-plan.md, Messages):
- in  text   {"type": "reset", "frame_interval": T}  episode start, T in s
- in  text   {"type": "frame", "id": n, "time": t}   then one binary message,
                                                     the JPEG
- out text   {"id": n}                               exactly one per frame,
                                                     in order (O4)
- out text   {"id": n, "error": "..."}               the frame broke the
                                                     728x544 JPEG contract
                                                     (O3); the orchestrator
                                                     fails the run

Rules:
- Times are simulated time (O2): `time` is the capture time the
  orchestrator sends (MuJoCo's data.time), never a wall clock.
- The window holds the last 6 marker frames (capture time + 4 slots); the
  7th append drops the oldest. Ages = capture time - newest capture time
  (newest 0, older negative).
- The first observation after a reset fills all six entries, at capture
  time, -T, ..., -5T, with the same slots: the rig starts at rest, so that
  history is true, and spacing it by T makes the differences read zero
  speed and zero acceleration instead of 0 / 0.
- A frame before any reset is a protocol error: there is no T to fill the
  window with.

observations.jsonl, one line per frame:
  {"id", "time", "slots": [[x, y] x4], "confidences": [c x4], "detect_ms",
   "window": [[age, x0, y0, x1, y1, x2, y2, x3, y3] x6]}
`window` runs oldest first; `confidences` and `detect_ms` are for checking,
not part of the marker frame.

usage: python synthetic_orin.py --orchestrator ws://... --out <dir>
                                --config <ObjectDetection.toml> --engine <file>
"""

import argparse
import collections
import json
import time
from dataclasses import dataclass
from pathlib import Path

from websockets.sync.client import connect

from detector import Detector, FrameError
from detector_config import DetectorConfig

# The loop's frame contract (synthetic-loop-dia.md, Contracts).
FRAME_SIZE = (728, 544)

# The policy input's window length (synthetic-loop-dia.md, Contracts).
WINDOW = 6


@dataclass(frozen=True)
class Entry:
    time: float  # capture time, simulated seconds
    slots: list[tuple[float, float]]


def window_rows(window: collections.deque) -> list[list[float]]:
    newest = window[-1].time
    return [[entry.time - newest, *(v for slot in entry.slots for v in slot)] for entry in window]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--orchestrator", required=True, help="the orchestrator's WebSocket URL")
    parser.add_argument("--out", type=Path, required=True, help="the run's output folder")
    parser.add_argument("--config", type=Path, required=True, help="ObjectDetection.toml")
    parser.add_argument("--engine", type=Path, required=True, help="TensorRT engine built by build_engine.py")
    args = parser.parse_args()

    detector = Detector(DetectorConfig.load(args.config), args.engine, FRAME_SIZE)
    window: collections.deque[Entry] = collections.deque(maxlen=WINDOW)
    interval = None  # T from the last reset; None before the first
    log_path = args.out / "observations.jsonl"
    # Line-buffered: a run that fails midway leaves every finished line.
    with open(log_path, "w", buffering=1) as log, connect(args.orchestrator, max_size=None) as ws:
        print(f"synthetic orin: connected to {args.orchestrator}, logging {log_path}", flush=True)
        for message in ws:
            if isinstance(message, bytes):
                raise RuntimeError("binary message without a frame header")
            header = json.loads(message)
            if header["type"] == "reset":
                interval = float(header["frame_interval"])
                window.clear()
                continue
            if header["type"] != "frame":
                raise RuntimeError(f"unknown message type {header['type']!r}")
            if interval is None:
                raise RuntimeError(f"frame {header['id']} before any reset")
            frame_id, capture = int(header["id"]), float(header["time"])
            jpeg = ws.recv()
            if not isinstance(jpeg, bytes):
                raise RuntimeError(f"frame {frame_id}: expected the JPEG, got text")

            started = time.perf_counter()
            try:
                marker_frame = detector.detect(jpeg)
            except FrameError as error:
                ws.send(json.dumps({"id": frame_id, "error": str(error)}))
                continue
            detect_ms = (time.perf_counter() - started) * 1000

            if not window:
                for k in range(WINDOW - 1, -1, -1):
                    window.append(Entry(capture - k * interval, marker_frame.slots))
            else:
                window.append(Entry(capture, marker_frame.slots))

            observation = {
                "id": frame_id,
                "time": capture,
                "slots": [list(slot) for slot in marker_frame.slots],
                "confidences": marker_frame.confidences,
                "detect_ms": round(detect_ms, 3),
                "window": window_rows(window),
            }
            log.write(json.dumps(observation) + "\n")
            ws.send(json.dumps({"id": frame_id}))
    detector.close()


if __name__ == "__main__":
    main()
