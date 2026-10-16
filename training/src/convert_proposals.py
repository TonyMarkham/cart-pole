"""Convert Ultralytics predict proposals to corpus-format labels.

Reads the 6-field proposal files (class x y w h conf) written by
`yolo detect predict save_txt save_conf` and writes 5-field corpus
labels (class x y w h) BESIDE each frame, keeping EVERY found box -
including duplicates - so class corrections are pure text edits:
change the leading class id, delete phantom lines.

Read-only everywhere else: frames are never opened for writing, and
the proposal files are left in place.
"""

from __future__ import annotations

from pathlib import Path

PROPOSALS = Path(r"D:\git\cart-pole\training\runs\real-preview\labels")
FRAMES_DIR = Path(r"C:\Users\tony\Downloads\run-1792167167")


def main() -> int:
    written = 0
    for proposal in sorted(PROPOSALS.glob("img_*.txt")):
        lines = []
        for line in proposal.read_text().splitlines():
            fields = line.split()
            if len(fields) >= 5:
                lines.append(" ".join(fields[:5]))
        if lines:
            (FRAMES_DIR / proposal.name).write_text("\n".join(lines) + "\n")
            written += 1
            classes = sorted({line.split()[0] for line in lines})
            print(f"{proposal.name}: {len(lines)} boxes, classes {classes}")
        else:
            print(f"{proposal.name}: EMPTY - skipped")
    print(f"{written} label files written to {FRAMES_DIR}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
