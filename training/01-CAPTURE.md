# 01 — Capture the synthetic corpus

**Path: 1 of 7. Before this: nothing. After this: 02-SCRIPTS.md.**

This step produces the synthetic training corpus with the Unity
player or a player build. One generation set = one run folder of
1000 labeled frames.

## What a run produces

```
datasets/generated/run_<Set>_<timestamp>/
  images/frame_NNNNNN.jpg   1456x1088 greyscale JPEG, quality 90
  labels/frame_NNNNNN.txt   class x_center y_center w h (normalized)
  markers.json              class map: 0=Left 376, 1=Right 596,
                            2=Pivot 920, 3=End 937
```

Full format and pipeline rules: `GENERATOR.md` at the repo root.

> ## Step 1 — author the generation sets
>
> Why: the sets are the experiment's independent variables — seed
> (replayability), state count (corpus size), and the camera/light
> ranges (how much real-world variation the detector must tolerate).
> Everything downstream is fixed by what is authored here.
>
> 1. In Unity: Create → Simulation/Generation Set. Author one set per
>    run: a fresh seed, `States` 1000, `Max Z` 3.5, camera/light/desk
>    ranges as required.
> 2. Wire the sets into the `SyntheticDataGenerator` component's
>    `m_Sets` list, in run order.
>
> Validation runs use small sets (Set-A: 5 states, Set-B: 6) so a full
> pass takes seconds. Production uses Set-01…Set-11.

> ## Step 2 — run the generator
>
> Why: this renders every drawn state, verifies each frame's labels
> against 8K crops, and writes the only copy of the corpus. There is no
> labeling step afterward — the labels exist because this step wrote
> them.
>
> 1. Player build (long runs): build with **Run in Background** enabled;
>    generation stalls when the window loses focus otherwise.
> 2. Editor (short validation runs only): press Play. The generator
>    exits play mode itself when all sets are done.

> ## Step 3 — read the log
>
> Why: the log is the only record that every set completed its full
> state count and that the verification gate did not abort — a set
> aborted mid-run leaves a partial folder that would silently shrink
> the training data.
>
> One line per set at start and end:
>
> ```
> [GenerationSetRunner] 'Set-01': 1000 states, seed 1001 -> <run folder>
> [GenerationSetRunner] 'Set-01': 1000 frames, 0 draws discarded on the Max Z cap, elapsed 00:03:13
> ```
>
> - `0 draws discarded` — no camera pose needed more frustum
>   compensation than Max Z allows.
> - A run that aborts with `consecutive frames proposed markers but the
>   gate confirmed none` means the detector is broken — delete the
>   partial run folder and stop.

> ## Step 4 — verify the output
>
> Why: `build_dataset.py` (doc 02) consumes every run folder it finds —
> a partial folder silently shrinks the training set and an unwanted
> one silently joins it. Verification here is the last cheap checkpoint.
>
> Per run folder: `images/` and `labels/` file counts must equal the
> set's `States`; `markers.json` must exist; nothing else is in the
> folder. Delete any folder you do not want in the training tree.

## Landed corpus (2026-10-16/17)

| Run | Seed | Frames | Boxes |
|---|---|---|---|
| Set-01 … Set-11 | 1001 … 1111 | 1000 each | 3867–3904 each |

Total: 11,000 frames, 42,769 boxes. Class share: Left 25.0%,
Right 25.0%, Pivot 25.7%, End 24.3%. Throughput: ~3:13 per 1000
states in a player build.

## Rules this step enforces

- Greyscale everywhere — no color render path exists.
- Every label line is verified against an 8K in-memory crop before it
  is written; a line survives only if the crop decodes to that
  class's tag. Never-wipe: the writer cannot empty a label file.
- One desk per frame; desks do not survive a set boundary.
- The camera pose is drawn from the set's absolute ranges, then
  dollied along Z until the RobotBounds fits the frustum. A draw
  needing more than Max Z is discarded and redrawn.
