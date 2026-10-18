"""Convert Ultralytics predict proposals to corpus-format labels.

Both pre-named real datasets (real-1440, real-640) are converted in
one run, plus the calibration dataset when it has proposals. For
each dataset: reads the 6-field proposal files (class
x y w h conf) written by that dataset's predict run
(training\\runs\\<run>\\labels) and writes 5-field corpus labels
(class x y w h) into datasets\\real-<n>\\data\\labels - one label per
frame in data\\images\\, keeping EVERY found box (including
duplicates) so class corrections are pure text edits: change the
leading class id, delete phantom lines.

A frame with no proposal file - or an empty one - gets an empty label
file: legal corpus state (no markers proposed; the human pass
decides). Frames are never opened for writing; proposal files are
left in place. No arguments: the datasets are fixed by the doc 06
workflow and never change.

The real-1440 dataset's labels are the scoring set - proposed by the
metrology model, corrected by the human. The real-640 dataset's
labels are the deployment-path proof: what the Pi's model generates
on the same capture, checked against that corrected truth (doc 06,
Step 8).
"""

from __future__ import annotations

from pathlib import Path

REAL = Path(r"D:\git\cart-pole\datasets")
RUNS = Path(r"D:\git\cart-pole\training\runs")
TREES = (
    ("real-1440", "1440"),
    ("real-640", "640"),
    ("calibration", "calibration"),
)


def convert(tree: str, run: str) -> tuple[int, int] | None:
    images = REAL / tree / "data" / "images"
    labels = REAL / tree / "data" / "labels"
    proposals = RUNS / run / "labels"

    if not images.is_dir():
        print(f"{tree}: images folder not found: {images}")
        print("land frames first (doc 06, Step 2) - tree skipped")
        return None
    if not proposals.is_dir():
        print(f"{tree}: proposals folder not found: {proposals}")
        print(f"run the {tree} predict first (doc 06) - tree skipped")
        return None

    labels.mkdir(parents=True, exist_ok=True)

    written = empty = 0
    for image in sorted(images.glob("*.jpg")):
        proposal = proposals / f"{image.stem}.txt"
        lines = []
        if proposal.exists():
            for line in proposal.read_text().splitlines():
                fields = line.split()
                if len(fields) >= 5:
                    lines.append(" ".join(fields[:5]))
        (labels / f"{image.stem}.txt").write_text(
            "\n".join(lines) + ("\n" if lines else "")
        )
        if lines:
            written += 1
            classes = sorted({line.split()[0] for line in lines})
            print(f"{tree} {image.stem}.txt: {len(lines)} boxes, classes {classes}")
        else:
            empty += 1
            print(f"{tree} {image.stem}.txt: EMPTY")
    print(f"{tree}: {written} label files written, {empty} empty, to {labels}")
    return written, empty


def main() -> int:
    results = [convert(tree, run) for tree, run in TREES]
    if all(result is None for result in results):
        print("nothing converted - no tree had both images and proposals")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
