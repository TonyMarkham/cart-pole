"""The brain's worker: the Python the Rust brain calls for the two things that
have to be Python on the robot, finding the markers (TensorRT) and the policy's
inference (`policy.py`), and nothing else (real-loop-dia.md, The run and idle
states, how the Rust brain uses Python).

The brain starts this as a child process when the brain itself starts, and it
lives as long as the brain. It loads the TensorRT engine and policy.npz once,
then speaks over its own two pipes, binary, in fixed sizes, so nothing is parsed
between a frame and its answer:

    first, one JSON line out: {"ready": true, "top_pulse_rate", "age_unit_ms",
                               "past_commands", "source"}
                           or {"error": "why it could not start"}
    then, until the input closes, either request:
      in  b"F" + the frame, 728 x 544 bytes, one per pixel, top row first
      out 20 little-endian f32: per class in class order, the box's centre x
          and y and its width and height, normalised to the frame, and its
          confidence; (-1, -1, 0, 0, confidence) for a marker not found
      in  b"P" + the policy's inputs, 36 little-endian f64
      out a, one little-endian f64

The frame is encoded as JPEG quality 90 and decoded again before detection,
because that is how the training frames were made (real-loop-dia.md, The pool on
the rig, difference 2). A frame the detector refuses answers every class not
found and says why on standard error: a failed frame ends nothing (The run and
idle states). Everything this prints goes to standard error, because standard
output is the brain's: at start the protocol takes a private copy of file
descriptor 1 and points descriptor 1 itself at standard error, so that
TensorRT's logger, which writes its warnings to the C library's stdout (seen on
the Orin, 2026-11-07: "[TRT] [W] Using an engine plan file across different
models of devices"), can never put a byte on the brain's pipe.

The settings are settings.toml beside this folder: [detector] for the engine,
[stream] for the frame size and the JPEG quality. The policy is saved/policy.npz.

usage (from __live__/orin/python, as the brain runs it): uv run python src/brain_worker.py
"""

import json
import os
import struct
import sys
from pathlib import Path
from typing import BinaryIO

import numpy as np

from detector_config import DEFAULT_PATH, DetectorConfig
from policy import ENTRIES, ROW, Policy, PolicyError

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

# The two requests' first bytes. Must match the brain's WORKER_FRAME_REQUEST and
# WORKER_POLICY_REQUEST (__live__/orin/rust/crates/brain/src/constants.rs).
FRAME_REQUEST = b"F"
POLICY_REQUEST = b"P"

# A detection's five numbers, as f32; the four classes' answer.
DETECTION = struct.Struct("<5f")
CLASSES = 4
NOT_FOUND = (-1.0, -1.0, 0.0, 0.0)

# Each request starts with one byte; the policy's inputs and its answer are
# little-endian f64, 8 bytes each (the brain's F64_BYTES).
REQUEST_BYTES = 1
F64_BYTES = 8
ACTION = struct.Struct("<d")

# The policy's shipped file, beside this folder.
POLICY_PATH = Path(__file__).resolve().parent.parent / "saved" / "policy.npz"

# The tables and keys read from settings.toml: they must match the names there.
# [stream] is the frame the detector gets and the JPEG quality it is encoded at;
# [detector] engine is the TensorRT engine file.
STREAM_TABLE = "stream"
WIDTH_KEY = "width"
HEIGHT_KEY = "height"
JPEG_QUALITY_KEY = "jpeg_quality"
DETECTOR_TABLE = "detector"
ENGINE_KEY = "engine"


def ready_line(policy: Policy) -> str:
    """The values the brain builds the inputs and the command with, from the
    policy file alone, so they are the trained run's."""
    return json.dumps({
        "ready": True,
        "top_pulse_rate": policy.top_pulse_rate,
        "age_unit_ms": policy.age_unit_ms,
        "past_commands": policy.past_commands,
        "source": policy.source,
    })


def error_line(reason: str) -> str:
    return json.dumps({"error": reason})


def pack_detections(slots, sizes, confidences) -> bytes:
    """The four classes' boxes as the brain reads them: 80 bytes."""
    out = bytearray()
    for slot, size, confidence in zip(slots, sizes, confidences):
        out += DETECTION.pack(slot[0], slot[1], size[0], size[1], confidence)
    return bytes(out)


def pack_not_found(confidences=None) -> bytes:
    """Every class not found, with the confidences if the model gave any."""
    confidences = confidences or [0.0] * CLASSES
    return b"".join(DETECTION.pack(*NOT_FOUND, c) for c in confidences)


def unpack_inputs(data: bytes, count: int) -> np.ndarray:
    """The brain's inputs, `count` little-endian f64."""
    return np.frombuffer(data, dtype="<f8", count=count).astype(np.float64)


def pack_action(a: float) -> bytes:
    return ACTION.pack(float(a))


def read_exact(inp: BinaryIO, size: int) -> bytes | None:
    """`size` bytes, or None once the input has closed."""
    data = bytearray()
    while len(data) < size:
        chunk = inp.read(size - len(data))
        if not chunk:
            return None
        data += chunk
    return bytes(data)


def to_jpeg(pixels: bytes, frame_size: tuple[int, int], quality: int) -> bytes:
    """The frame as the detector takes it: encoded at the training frames' quality."""
    import cv2  # the Orin's OpenCV; imported here so the framing tests need none

    width, height = frame_size
    grey = np.frombuffer(pixels, dtype=np.uint8).reshape(height, width)
    ok, encoded = cv2.imencode(".jpg", grey, [cv2.IMWRITE_JPEG_QUALITY, quality])
    if not ok:
        raise ValueError("the frame did not encode as a JPEG")
    return encoded.tobytes()


def serve(inp: BinaryIO, out: BinaryIO, detect, act, frame_bytes: int, inputs: int) -> None:
    """Answers requests until the input closes. `detect(pixels) -> bytes` gives the
    80-byte answer for a frame; `act(inputs) -> float` gives a."""
    while True:
        request = inp.read(REQUEST_BYTES)
        if not request:
            return
        if request == FRAME_REQUEST:
            pixels = read_exact(inp, frame_bytes)
            if pixels is None:
                return
            out.write(detect(pixels))
        elif request == POLICY_REQUEST:
            data = read_exact(inp, inputs * F64_BYTES)
            if data is None:
                return
            out.write(pack_action(act(unpack_inputs(data, inputs))))
        else:
            raise ValueError(f"unknown request byte {request!r}")
        out.flush()


def protocol_pipe() -> BinaryIO:
    """The brain's pipe, kept for the protocol alone: a private copy of file
    descriptor 1, after which descriptor 1 is made another standard error, so
    anything else that writes to stdout, Python's print or TensorRT's C++ logger,
    lands beside the worker's own messages and never on the pipe."""
    private = os.dup(sys.stdout.fileno())
    os.dup2(sys.stderr.fileno(), sys.stdout.fileno())
    return os.fdopen(private, "wb")


def main() -> int:
    out = protocol_pipe()
    try:
        settings = tomllib.loads(DEFAULT_PATH.read_text())
        stream = settings[STREAM_TABLE]
        frame_size = (int(stream[WIDTH_KEY]), int(stream[HEIGHT_KEY]))
        quality = int(stream[JPEG_QUALITY_KEY])
        policy = Policy.load(POLICY_PATH)
        config = DetectorConfig.load()
        from detector import Detector, FrameError  # TensorRT: the Orin only

        detector = Detector(config, Path(settings[DETECTOR_TABLE][ENGINE_KEY]).expanduser(), frame_size)
    except (OSError, KeyError, ValueError, PolicyError, RuntimeError, ImportError) as error:
        out.write((error_line(f"{type(error).__name__}: {error}") + "\n").encode())
        out.flush()
        return 1
    print(f"brain worker: engine loaded, policy {policy.source}", file=sys.stderr, flush=True)
    out.write((ready_line(policy) + "\n").encode())
    out.flush()

    def detect(pixels: bytes) -> bytes:
        try:
            frame = detector.detect(to_jpeg(pixels, frame_size, quality))
        except (FrameError, ValueError) as error:
            print(f"brain worker: frame failed: {error}", file=sys.stderr, flush=True)
            return pack_not_found()
        return pack_detections(frame.slots, frame.sizes, frame.confidences)

    def act(inputs: np.ndarray) -> float:
        return float(policy.action(inputs))

    try:
        serve(sys.stdin.buffer, out, detect, act, frame_size[0] * frame_size[1], ENTRIES * ROW + policy.past_commands)
    finally:
        detector.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
