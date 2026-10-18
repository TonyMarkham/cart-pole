"""Gate the generated corpus and write the tree's data.yaml.

Reads datasets/generated — the tree the Unity generator writes (one
run_<set>_<timestamp> folder per generation set, images/ + labels/ +
markers.json inside) — runs the derivable quality gate over every
run, then writes data.yaml at the tree root listing whole runs as
train/val with the class map read from the runs' markers.json.

Nothing is copied or converted: the tree's layout is what Ultralytics
scans natively (images/ and labels/ siblings paired by stem), so the
yaml points at run folders and training reads them in place. The old
regime's hardlink-merge step is gone with the layout it served.

Stdlib only. Run through uv from training/: uv run python src\\build_dataset.py
"""

from __future__ import annotations
from pathlib import Path

import argparse
import json
import sys

TRAINING = Path(__file__).resolve().parents[1]
REPO = TRAINING.parent
TREE = REPO / "datasets" / "generated"

VALID_CLASSES = {0, 1, 2, 3}


def discover_runs(tree: Path) -> list[Path]:
    """Every run folder in timestamp order - the zero-padded run_*
    names sort chronologically."""

    return sorted(p for p in tree.iterdir() if p.is_dir())


def gate_run(name: str, run: Path) -> tuple[int, int, list[str]]:
    """Pair frames with labels in images/ and labels/. Returns
    (frames, boxes, problems); a non-empty problems list refuses the
    build."""

    images = run / "images"
    labels = run / "labels"
    stem_jpg = {p.stem for p in images.glob("*.jpg")}
    stem_txt = {p.stem for p in labels.glob("*.txt")}
    problems: list[str] = []

    for stem in sorted(stem_jpg - stem_txt):
        problems.append(f"{name}: {stem}.jpg has no label.")

    for stem in sorted(stem_txt - stem_jpg):
        problems.append(f"{name}: {stem}.txt has no frame.")

    frames = 0
    boxes = 0
    empty = 0

    for txt in sorted(labels.glob("*.txt")):
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
                    f"{name}: labels/{txt.name}:{lineno} - expects 5 fields, "
                    f"got {len(fields)}")
            if ok:
                try:
                    cls = int(fields[0])
                    coords = [float(v) for v in fields[1:]]
                except ValueError:
                    problems.append(
                        f"{name}: labels/{txt.name}:{lineno} - unparseable: "
                        f"{line!r}")

                    ok = False

            if ok and cls not in VALID_CLASSES:
                problems.append(
                    f"{name}: labels/{txt.name}:{lineno} - class {cls} "
                    "outside 0-3")

                ok = False

            if ok and not all(0.0 <= v <= 1.0 for v in coords):
                problems.append(
                    f"{name}: labels/{txt.name}:{lineno} - coordinate "
                    "outside [0, 1]")

                ok = False

            if ok:
                boxes += 1

    if empty:
        print(f"{name}: {empty} empty label files "
              "(no markers confirmed in frame)")

    return frames, boxes, problems


def write_yaml(tree: Path, train: list[Path], val: list[Path]) -> Path:
    """data.yaml at the tree root, exactly the default keys. `path:` is
    omitted - it defaults to the yaml's own directory, so the run
    entries are written relative to it. `names:` comes from the runs'
    markers.json (the rig slot order, read from disk, never retyped);
    every run carries the same map, so the val run's copy is the
    source."""

    source = val[0] if val else train[0]
    markers = json.loads((source / "markers.json").read_text())
    entries = sorted(markers["markers"], key=lambda m: m["classId"])
    lines = ["train:", *[f"- {p.name}" for p in train]]
    lines += ["val:", *[f"- {p.name}" for p in val]]
    lines += [
        "names:",
        *[f"- {e['label']}" for e in entries],
        "channels: 1",
    ]
    out = tree / "data.yaml"
    out.write_text("\n".join(lines) + "\n")
    return out


def main() -> int:
    """Gate the whole tree, then write the yaml. Returns the process
    exit code: 0 written, 1 refused or misconfigured."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--val", metavar="NAME", default=None,
        help="run folder (or substring) held out for validation; "
        "default: the newest run")
    args = parser.parse_args()

    if not TREE.is_dir():
        print(f"tree not found: {TREE}")

        return 1

    runs = discover_runs(TREE)
    runs = [r for r in runs if (r / "images").is_dir()]
    if not runs:
        print(f"no run folders under {TREE}")

        return 1

    if args.val:
        val = [r for r in runs if args.val in r.name]
        train = [r for r in runs if r not in val]
    else:
        train, val = runs[:-1], runs[-1:]

    # Gate everything before writing a single yaml line: a training
    # run is hours - a corrupt frame aborts here, cheaply.
    all_problems: list[str] = []
    total_frames = 0
    total_boxes = 0

    for run in [*train, *val]:
        frames, boxes, problems = gate_run(run.name, run)
        all_problems += problems
        total_frames += frames
        total_boxes += boxes

    if all_problems:
        print(f"BUILD REFUSED - {len(all_problems)} problem(s):")
        for problem in all_problems:
            print(f"  {problem}")

        return 1

    print(f"train: {len(train)} runs, "
          f"{sum(1 for p in train for _ in (p / 'images').glob('*.jpg'))} frames")
    print(f"val:   {len(val)} runs, "
          f"{sum(1 for p in val for _ in (p / 'images').glob('*.jpg'))} frames")

    yaml_path = write_yaml(TREE, train, val)
    print(f"dataset: {total_frames} frames, {total_boxes} boxes")
    print(f"yaml: {yaml_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
