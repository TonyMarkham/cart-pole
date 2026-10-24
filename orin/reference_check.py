"""Does the detector compute what Ultralytics' own predictor computes on the
same model and frames? (synthetic-orin-plan.md Done-when 1.)

Reads a reference CSV written by the training env's predictor run (one row
per frame: `frame`, then per class `<Name>_conf` - empty when not detected -
and `<Name>_xy`, the normalized center "x y"), runs the detector on each
frame, and checks every slot: the same detected / not-detected decision,
centers within 1 px, confidences within 0.01. A decision may differ when the
confidence lies within 0.01 of the threshold (the reference's, or ours when
the reference has none); such slots are listed, not failed. Exit 1 on any
failure. The same rules as the Rust crate's reference_check example.

Time per frame is decode + letterbox + copy + infer + slot rule, after a
20-frame warm-up.

usage: python reference_check.py <ObjectDetection.toml> <engine> <reference.csv> <frames_dir>
"""

import csv
import statistics
import sys
import time
from pathlib import Path

from detector import NOT_DETECTED, Detector
from detector_config import DetectorConfig

# Done-when 1's tolerances (detector-plan.md Done-when 2).
CENTER_PX = 1.0
CONFIDENCE = 0.01

# The loop's frame contract (synthetic-loop-dia.md, Contracts).
FRAME_SIZE = (728, 544)


def main() -> int:
    config_path, engine_path, csv_path, frames_dir = map(Path, sys.argv[1:])
    config = DetectorConfig.load(config_path)
    detector = Detector(config, engine_path, FRAME_SIZE)
    w, h = FRAME_SIZE

    with open(csv_path, newline="") as f:
        rows = list(csv.DictReader(f))
    first = (frames_dir / rows[0]["frame"]).read_bytes()
    for _ in range(20):  # warm-up: clocks and lazy CUDA init
        detector.detect(first)

    failures, borderline, times = [], [], []
    max_dx = max_dy = max_dconf = 0.0
    for row in rows:
        jpeg = (frames_dir / row["frame"]).read_bytes()
        t0 = time.perf_counter()
        frame = detector.detect(jpeg)
        times.append((time.perf_counter() - t0) * 1000)
        for name, slot, conf in zip(config.classes, frame.slots, frame.confidences):
            ref_conf = float(row[f"{name}_conf"]) if row[f"{name}_conf"] else None
            ours = slot != NOT_DETECTED
            where = f"{row['frame']} {name}: ours conf {conf:.4f}, reference {ref_conf if ref_conf is not None else 'not detected'}"
            if ours != (ref_conf is not None):
                near = abs((ref_conf if ref_conf is not None else conf) - config.confidence) <= CONFIDENCE
                (borderline if near else failures).append(where)
                continue
            if ref_conf is None:
                continue
            rx, ry = map(float, row[f"{name}_xy"].split())
            dx, dy, dconf = abs(slot[0] - rx) * w, abs(slot[1] - ry) * h, abs(conf - ref_conf)
            max_dx, max_dy, max_dconf = max(max_dx, dx), max(max_dy, dy), max(max_dconf, dconf)
            if dx > CENTER_PX or dy > CENTER_PX or dconf > CONFIDENCE:
                failures.append(f"{where}; center off {dx:.3f} / {dy:.3f} px")
    detector.close()

    print(f"{len(rows)} frames, {len(rows) * len(config.classes)} slots: {len(failures)} failures, "
          f"{len(borderline)} borderline; worst center {max_dx:.3f} / {max_dy:.3f} px, worst conf {max_dconf:.4f}")
    print(f"detect ms/frame: mean {statistics.mean(times):.2f}, median {statistics.median(times):.2f}, "
          f"min {min(times):.2f}, max {max(times):.2f}")
    for line in borderline:
        print("  borderline " + line)
    for line in failures:
        print("  FAIL " + line)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
