# 07 — Refining the models with real-world data

**Path: 7 of 7. Before this: 06-REAL_LABELING.md (a clean evaluation).
Conditional: this doc is read only when the evaluation shows
systematic divergence.**

## Premise

The varied synthetic corpus was built to train the models to
readiness **without real-world training**, and doc 06 is the standing
test of that premise. This doc exists so the refinement path is
documented *before* it is needed — not because it is expected.

**The premise has already held once, end to end.** The first corpus
FAILED the real-world test — the Right→End position shortcut, learned
from a fixed-camera corpus. The refinement applied was not real-world
post-training: the **synthetic generation itself was altered** (camera
variation, new generation sets), the object training was run again,
and the second real evaluation came back clean. That loop — evaluation
fails → the generator changes → retrain → evaluate again — is the
workflow of record, and it met the initial expectations. Rung 3 below
is the demonstrated path; the real-data rungs exist but have never
been used.

**The owner's accuracy bar is met (2026-10-18):** the full 146-frame
deployment evaluation came back 100% clean, and the owner declares
the model as accurate as needed — **hand labeling is not required**.
Rung 1's corrections will not exist, which also retires the one
converter caveat: with no hand edits anywhere, re-converting any tree
is always idempotent and safe. The ladder below stays documented as
the contingency it is — built only if a future evaluation ever
demands it.

Every step here is built only as far as the evidence demands.

## When refinement is warranted

Read the signal, not the noise:

- **Isolated mistakes** — one wrong box on one frame: fix the label,
  note it, move on. Every dataset has noise; noise is not a training
  problem.
- **Systematic divergence** — the same mistake recurring across
  frames: one class consistently wrong, one marker consistently
  missed, phantoms in consistent positions. This is what warrants
  refinement, and the ladder below is ordered cheapest first.

## The refinement ladder

> ## Rung 1 — correct the labels (the record)
>
> Why: real frames have no ground truth — the human is the class
> arbiter. The corrections are not drudgery to minimize; they are the
> measurement of the sim-to-real gap, so every edit is counted.
>
> Open each frame (or its annotated preview from 06, Step 3) and edit
> the txt in `real-1440\data\labels\`:
>
> - **Wrong class on a correct box**: change the leading id.
> - **Phantom boxes** (duplicates of one physical marker): delete the
>   line.
> - **Missing markers**: add a line in corpus format.
>
> **Log the corrections — they are the measurement.** Count per class:
> boxes accepted as-is, moved, deleted, added by hand. That
> distribution is the quantified sim-to-real gap.
>
> Known corrections to expect from the first batch (70 frames,
> 2026-10-16 — the OLD corpus's model, not the current one):
>
> - The **rail-right reference marker** (physically Right/596) is
>   classified **End** in most frames — flip those to Right.
> - The **cart pivot marker** (Pivot/920) goes undetected or is called
>   **End** near the rail-right marker — re-add or flip.
> - **Duplicate Ends** are pole-tip + misclassified rail-right (+ sometimes
>   pivot) — keep one per physical marker.
> - **Left** was correct in all 70 frames.
>
> These were the failures the varied corpus was built to kill — the
> current evaluation (06) says they are dead at first look. The
> corrections log from any new batch is the test of whether they stay
> dead.

> ## Rung 2 — score against corrected truth
>
> Why: model-vs-model agreement (06, Step 6's optional check) is not
> truth-grounded. Once the 1440 labels are human-corrected, the same
> command produces a real number — the deployment model's precision
> and recall against verified truth.
>
> ```powershell
> uv run yolo detect val model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx data=D:/git/cart-pole/datasets/real-1440/data.yaml imgsz=640
> ```
>
> `data=` points at the scoring dataset's yaml (06, Step 6). **Bar
> (adjustable): per-class AP50 ≥ 0.90, no class below 0.80.**

> ## Rung 3 — feed the misses back into the synthetic corpus
>
> Why: real data is scarce and the generator is rich. A systematic
> miss usually means a *situation* the corpus never drew — a pose,
> occlusion, or lighting range the generation sets don't cover. The
> cheapest fix is almost always a new generation set, not new
> training data.
>
> - Every logged miss maps to a generation-set change: a wider range,
>   a forced situation, a new desk/light combination.
> - Regenerate, retrain (04-COMMANDS.md), re-evaluate (06). The loop
>   is the same one that produced the current models.

> ## Rung 4 — fine-tune with real frames
>
> Why: when the generator cannot reproduce what the rig produces, the
> corrected real labels themselves become training data — a small
> fine-tune from `best.pt`, not a from-scratch run.
>
> - The corrected `real-1440` labels are corpus-format; they drop
>   into the training tree as-is.
> - Keep it small: a fine-tune takes the real frames as a seasoning
>   pass, not a diet. Watch the synthetic val score for regression.

> ## Rung 5 — the calibration-phase design (decided, parked)
>
> Why: if a Right→End-class confusion survives rungs 3 and 4, the
> escalation is already designed — persistent Left/Right reference
> markers calibrated per session, and a hard gate that any detected
> End must sit 500 mm ± ε from Pivot (the pole-length arc).
>
> That spec is decided; it is built only if the measurements demand
> it. It has never been demanded.

## What the findings mean

The first real evaluation (2026-10-17) came back clean: all four
markers, correct classes, no phantoms — including Right on the
rail-right marker, the old failure. The premise held at first look.

If a future batch diverges systematically, this doc's ladder runs
cheapest-first, and each rung's output is recorded: corrections
counted, scores kept, corpus changes named. The refinement loop ends
when the evaluation in 06 comes back clean again — the premise
re-confirmed, one level stronger.
