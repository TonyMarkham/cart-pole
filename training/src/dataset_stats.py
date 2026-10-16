"""Statistics for the assembled dataset - pure text parsing, no image
decoding. Reports per-class box counts and box-size histograms read
from the label files; the pixel numbers that justified imgsz=1440
come from here.
"""

from __future__ import annotations
from pathlib import Path

TRAINING = Path(__file__).resolve().parents[1]
DATASET = TRAINING / "datasets" / "cart-pole"
IMG_WIDTH = 1440
IMG_HEIGHT = 1080

def bucket_stats(bucket: str) -> tuple[int, int, dict[int, int], dict[str, int], list[float]]:
    """Count label files, boxes, boxes per class, and boxes per run.
    The run name comes free from the filename prefix the build wrote:
    <run>_frame_NNNNNN.txt."""

    files = 0
    boxes = 0
    per_class = {0: 0, 1: 0, 2: 0, 3: 0}
    per_run: dict[str, int] = {}
    sizes: list[float] = []

    for txt in sorted((DATASET / "labels" / bucket).glob("*.txt")):
        files += 1

        for line in txt.read_text().splitlines():
            if not line.strip():
                continue

            boxes += 1
            fields = line.split()
            cls = int(fields[0])
            per_class[cls] += 1
            sizes.append(float(fields[3]) * IMG_WIDTH)
            run = txt.name.split("_frame_")[0]
            per_run[run] = per_run.get(run, 0) + 1

    return files, boxes, per_class, per_run, sizes

if __name__ == "__main__":
    for bucket in ("train", "val"):
        files, boxes, per_class, per_run, sizes = bucket_stats(bucket)

        print(f"--- {bucket}: {files} files, {boxes} boxes")

        for cls, count in sorted(per_class.items()):
            print(f"    class {cls}: {count}")

        for run, count in sorted(per_run.items()):
            print(f"    {run}: {count}")

        print(f"    width px: min {min(sizes):.1f}  max {max(sizes):.1f}  "
              f"mean {sum(sizes) / len(sizes):.1f}")

        hist: dict[int, int] = {}

        for w in sizes:
            bin_index = int(w // 4)
            hist[bin_index] = hist.get(bin_index, 0) + 1

        for bin_index in sorted(hist):
            print(f"    {bin_index * 4}-{bin_index * 4 + 3}px: {hist[bin_index]}")
