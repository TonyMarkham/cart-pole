#!/usr/bin/env python3
"""Verify YOLO labels against OpenCV ArUco detection.

The markers are 4x4 ArUco tags from DICT_4X4_1000. The authoritative
class-to-tag mapping comes from the dataset's markers.json, written by
the capture from the rig's authored MarkerSOs (class = rig slot order,
tag id = the SO's arUcoId). A label box survives only if OpenCV detects
the marker id that box's class maps to, overlapping the box (IoU >= 0.5)
- a box pointing at an invisible or misidentified marker is dropped.

Usage:
    python Tools/aruco_check.py <dataset_dir>            # dry run
    python Tools/aruco_check.py <dataset_dir> --write    # rewrite labels

Requires: pip install opencv-python  (or: uv run --with opencv-python)
"""
import json
import sys
from pathlib import Path

import cv2

DETECTOR_PARAMS = cv2.aruco.DetectorParameters()
DETECTOR_PARAMS.minMarkerPerimeterRate = 0.01
DETECTOR_PARAMS.cornerRefinementMethod = cv2.aruco.CORNER_REFINE_SUBPIX
DETECTOR = cv2.aruco.ArucoDetector(
    cv2.aruco.getPredefinedDictionary(cv2.aruco.DICT_4X4_1000),
    DETECTOR_PARAMS)

CROP_SCALES = (1, 2, 4)


def detect_ids(image):
    corners, ids, _ = DETECTOR.detectMarkers(image)
    return ids.flatten().tolist() if ids is not None else []


def crop_confirms(crop, expected_id):
    for scale in CROP_SCALES:
        up = cv2.resize(crop, None, fx=scale, fy=scale,
                        interpolation=cv2.INTER_NEAREST)
        if expected_id in detect_ids(up):
            return True
    return False


def main():
    directory = Path(sys.argv[1])
    write = "--write" in sys.argv
    mapping_path = directory / "markers.json"
    mapping = {}
    labels = {}
    if mapping_path.exists():
        for m in json.loads(mapping_path.read_text())["markers"]:
            mapping[int(m["classId"])] = int(m["id"])
            labels[int(m["classId"])] = m["label"]
    else:
        print("WARNING: no markers.json - falling back to IoU-only checks")

    # The per-marker crops are retained by Unity itself (cut from the
    # 8K frame using the exact projected marker positions) - Python
    # does not crop. Verification reads those retained crops directly.

    total_kept, total_dropped = 0, 0
    for jpg in sorted(directory.glob("frame_*.jpg")):
        if "_marker_" in jpg.stem or jpg.stem.endswith("_8k"):
            continue
        txt = jpg.with_suffix(".txt")
        if not txt.exists():
            continue
        kept, dropped = [], []
        for line in txt.read_text().splitlines():
            parts = line.split()
            if len(parts) != 5:
                continue
            cls = int(parts[0])
            expected = mapping.get(cls)
            confirmed = False
            if expected is not None:
                crop_path = directory / f"{jpg.stem}_8k_marker_{expected:04d}.jpg"
                if crop_path.exists():
                    confirmed = crop_confirms(
                        cv2.imread(str(crop_path)), expected)
            if confirmed:
                kept.append(line)
            else:
                reason = ("no verified crop" if expected is not None else
                          "no mapping for class")
                dropped.append(f"{line} ({reason})")
        total_kept += len(kept)
        total_dropped += len(dropped)
        dropped_labels = [labels.get(int(l.split()[0]), "?")
                          for l in dropped]
        dropped_text = ", ".join(dropped_labels) if dropped_labels else "none"
        print(f"{jpg.name}: {len(kept)} confirmed, {len(dropped)} dropped "
              f"[{dropped_text}]")
        print(f"dropped_markers: {dropped_text}")
        for d in dropped:
            print(f"    dropped: {d}")
        if write:
            txt.write_text("".join(line + "\n" for line in kept))
    print(f"done: {total_kept} kept, {total_dropped} dropped "
          f"({'labels rewritten' if write else 'dry run - use --write to apply'})")


if __name__ == "__main__":
    main()
