# 08 — convert_proposals.py, explained

## The problem it solves

Your 70 real frames have no labels. Something has to create them. The
detector already produced draft boxes for every frame — but it saved
them in its own format (6 fields per line, with a confidence score) in
its own folder (`training\runs\real-preview\labels\`), not in the
format your datasets use, and not next to your images.

The script moves those drafts into the exact shape your synthetic
datasets use, so the real frames become a normal, usable dataset —
with you then correcting what the detector got wrong.

## What it reads

`D:\git\cart-pole\training\runs\real-preview\labels\img_0000.txt` …
`img_0069.txt`

These are the 70 detector drafts. Each line currently has 6 fields:
`class, x_center, y_center, width, height, confidence`.

## What it writes

`C:\Users\tony\Downloads\run-1792167167\img_0000.txt` … `img_0069.txt`

Seventy **new** files, one sitting beside each of your frames. Each
line now has 5 fields — `class, x_center, y_center, width, height` —
the confidence column stripped. That is byte-for-byte the same format
your synthetic labels use.

## What it keeps

**Every box the detector found, unchanged in position and size.**
Nothing is deduplicated, nothing is reclassified, nothing is deleted.
If a frame has three "End" boxes, the label file has three lines. Your
review is a pure text edit: change a leading class id, or delete a
line that shouldn't exist.

## What it never touches

The 70 JPGs — never opened for writing. The proposal files —
read-only, left in place. The corpus, the runs, anything else on any
drive. The only files it creates are the 70 new txt files, none of
which exist today, so nothing can be overwritten.

## What you see when you run it

One console line per frame, like:

```
img_0000.txt: 4 boxes, classes ['0', '2', '3']
```

— box count and which class ids are present, so you know what to
expect before opening the file. Then a final count:
`70 label files written`.

## How to run it

```powershell
uv run python src\convert_proposals.py
```

from `D:\git\cart-pole\training\`. No arguments — the two directories
are constants at the top of the file, and they're the ones you already
know.

## What the world looks like after

Seventy real frames, each with a label file in corpus format beside it
— the same layout as your training data. The class ids inside will
contain the detector's known mistakes (rail-right boxes marked End,
some duplicates), and those are your edits to make: fix the id, or
delete the line. When your corrections are done, that folder is the
real-frame test set, ready to score.
