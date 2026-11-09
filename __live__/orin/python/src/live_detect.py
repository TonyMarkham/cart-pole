"""Shows whether the detector can see the markers through the real camera, with no
page or service in the way: prints where each marker is, frame by frame, and saves
one frame to look at.

Runs on the Orin only. Nothing moves: it only watches.

The settings are settings.toml, beside this folder.

usage (from __live__/orin/python): uv run python src/live_detect.py <frames> [frame.jpg]
"""

import statistics
import sys
import time
from pathlib import Path

import cv2
import numpy as np

from camera import Camera
from detector import NOT_DETECTED, Detector
from detector_config import DEFAULT_PATH, DetectorConfig

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib


def main() -> int:
    wanted = int(sys.argv[1])
    saved = Path(sys.argv[2]) if len(sys.argv) > 2 else None
    settings = tomllib.loads(DEFAULT_PATH.read_text())
    stream = settings["stream"]
    width, height = stream["width"], stream["height"]
    config = DetectorConfig.load()
    engine = Path(settings["detector"]["engine"]).expanduser()
    detector = Detector(config, engine, (width, height))
    found = {name: 0 for name in config.classes}
    times = []
    done = 0
    started = time.perf_counter()
    with Camera(settings["camera"], stream) as camera:
        for index, pixels in enumerate(camera.frames()):
            if index < stream["settle_frames"]:
                started = time.perf_counter()
                continue
            grey = np.frombuffer(pixels, np.uint8).reshape(height, width)
            t0 = time.perf_counter()
            ok, jpeg = cv2.imencode(".jpg", grey, [cv2.IMWRITE_JPEG_QUALITY, stream["jpeg_quality"]])
            if not ok:
                raise RuntimeError("the frame did not encode as a JPEG")
            frame = detector.detect(jpeg.tobytes())
            times.append((time.perf_counter() - t0) * 1000)
            parts = []
            for name, slot, conf in zip(config.classes, frame.slots, frame.confidences):
                if slot == NOT_DETECTED:
                    parts.append(f"{name} -")
                else:
                    found[name] += 1
                    parts.append(f"{name} {slot[0] * width:7.2f},{slot[1] * height:7.2f} ({conf:.2f})")
            print(f"{done:5d} " + "  ".join(parts), flush=True)
            if saved is not None and done == 0:
                saved.write_bytes(jpeg.tobytes())
            done += 1
            if done == wanted:
                break
    elapsed = time.perf_counter() - started
    detector.close()
    print(f"{done} frames in {elapsed:.2f} s: {done / elapsed:.1f} frames/s. Found: "
          + ", ".join(f"{name} {count}" for name, count in found.items()))
    print(f"encode + detect ms/frame: mean {statistics.mean(times):.2f}, max {max(times):.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
