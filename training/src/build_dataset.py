"""Assemble the Ultralytics dataset from the capture corpus.

Reads datasets/cart-pole at the repo root (pilot frames flat, bulk runs
in run_<timestamp> children), gates every label through the derivable
checks, then hardlinks frames and labels into the train/val layout
under training/datasets/cart-pole. Stdlib only.
"""

from __future__ import annotations
from pathlib import Path

import json
import os
import shutil
import sys

TRAINING = Path(__file__).resolve().parents[1]
REPO = TRAINING.parent
CORPUS = REPO / "datasets" / "cart-pole"
OUT = TRAINING / "datasets" / "cart-pole"

VALID_CLASSES = {0, 1, 2, 3}

def discover_runs(corpus: Path) -> list[tuple[str, Path]]:
    """Every capture location in stable order: the flat pilot corpus
    first, then the run children in timestamp order."""

    runs: list[tuple[str, Path]] = []

    if any(corpus.glob("frame_*.jpg")):
        runs.append(("pilot-flat", corpus))

    runs +=[(p.name, p) for p in sorted(corpus.iterdir()) if p.is_dir()]

    return runs

def gate_run(name: str, run: Path) -> tuple[int, int, list[str]]:
    """Pair frames with labels. Returns (frames, boxes, problems); a
    non-empty problems list refuses the build."""

    stem_jpg = {p.stem for p in run.glob("frame_*.jpg")}
    stem_txt = {p.stem for p in run.glob("frame_*.txt")}
    problems: list[str] = []

    for stem in sorted(stem_jpg - stem_txt):
        problems.append(f"{name}: {stem}.jpg has no label.")

    for stem in sorted(stem_txt - stem_jpg):
        problems.append(f"{name}: {stem}.txt has no frame.")

    frames = 0
    boxes = 0
    empty = 0

    for txt in sorted(run.glob("frame_*.txt")):
        frames += 1
        lines = [l for l in txt.read_text().splitlines() if l.strip()]
        if not lines:
            empty += 1
            continue

        for lineno, line in enumerate(lines, 1):
            fields = line.split()
            ok = len(fields) == 5
            if not ok:
                problems.append(
                    f"{name}: {txt.name}:{lineno} - expects 5 fields, "
                    f"got {len(fields)}")
            if ok:
                try:
                    cls = int(fields[0])
                    coords = [float(v) for v in fields[1:]]
                except ValueError:
                    problems.append(
                        f"{name}: {txt.name}:{lineno} - unparseable: "
                        f"{line!r}")

                    ok = False

            if ok and cls not in VALID_CLASSES:
                problems.append(
                    f"{name}: {txt.name}:{lineno} - class {cls} outside 0-3")

                ok = False

            if ok and not all(0.0 <= v <= 1.0 for v in coords):
                problems.append(
                    f"{name}: {txt.name}:{lineno} - coordinate outside [0, 1]")

                ok = False

            if ok:
                boxes += 1

    if empty:
        print(f"{name}: {empty} empty label files "
              "(all markers behind camera)")

    return frames, boxes, problems

def place(src: Path, dst: Path) -> None:
    """Hardlink the artifact into the dataset; fall back to a copy when
    the filesystem refuses. A hardlink is a second directory entry for
    the SAME file - one copy of the bytes, two names."""

    try:
        os.link(src, dst)
    except OSError:
        shutil.copy2(src, dst)

def reset_output() -> None:
    """Wipe and recreate the generated tree. The output is a build
    artifact - fully regenerable from the corpus - so every build
    starts clean instead of mixing fresh files with stale ones. The
    corpus directory is only ever read."""

    for sub in ("images/train", "images/val", "labels/train", "labels/val"):
        target = OUT / sub
        if target.exists():
            shutil.rmtree(target)
        target.mkdir(parents=True, exist_ok=True)

def write_yaml() -> None:
    """cart-pole.yaml is built from the corpus's markers.json - the
    class map is the rig slot order, read from disk, never retyped."""

    markers = json.loads((CORPUS / "markers.json").read_text())
    entries = sorted(markers["markers"], key=lambda m: m["classId"])
    lines = [
        f"path: {OUT.as_posix()}",
        "train: images/train",
        "val: images/val",
        "names:",
    ]
    lines += [f'  {e["classId"]}: {e["label"]}' for e in entries]
    (OUT / "cart-pole.yaml").write_text("\n".join(lines) + "\n")

def main() -> int:
    """Gate the whole corpus, then build. Returns the process exit
    code: 0 built, 1 refused or misconfigured."""

    if not CORPUS.is_dir():
        print(f"corpus not found: {CORPUS}")

        return 1

    runs = discover_runs(CORPUS)
    # Gate everything before placing a single file: a build happens
    # only when the whole corpus parses clean.
    gated: list[tuple[str, Path, str, int, int]] = []
    all_problems: list[str] = []
    total_frames = 0
    total_boxes = 0

    for name, run in runs:
        frames, boxes, problems = gate_run(name, run)
        # Split by whole run - frames within a run are near-duplicates
        # (same pools, same seed pair) and mixing them into both
        # buckets would inflate val. The flat pilot is the odd run out
        # and lands in val; the ten bulk runs land in train.
        bucket = "val" if name == "pilot-flat" else "train"
        gated.append((name, run, bucket, frames, boxes))
        all_problems += problems
        total_frames += frames
        total_boxes += boxes

    if all_problems:
        print(f"BUILD REFUSED - {len(all_problems)} problem(s):")
        for problem in all_problems:
            print(f"  {problem}")

        return 1

    reset_output()

    for name, run, bucket, frames, boxes in gated:
        prefix = f"{name}_"

        for jpg in sorted(run.glob("frame_*.jpg")):
            place(jpg, OUT / "images" / bucket / (prefix + jpg.name))

        for txt in sorted(run.glob("frame_*.txt")):
            place(txt, OUT / "labels" / bucket / (prefix + txt.name))

        print(f"{name}: {frames} frames, {boxes} boxes -> {bucket}")

    write_yaml()
    print(f"dataset: {total_frames} frames, {total_boxes} boxes")
    print(f"yaml: {OUT / 'cart-pole.yaml'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

