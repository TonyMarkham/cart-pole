# 05 — Export, validate, and read the results

**Path: 5 of 7. Before this: 04-COMMANDS.md (all steps). After this:
06-REAL_LABELING.md.**

Training produced two models. This doc exports the deployment model,
proves the export, and records what the numbers decided.

> ## Step 1 — export the deployment model to ONNX
>
> Why: the Pi's CPU runtime consumes ONNX, not PyTorch checkpoints.
>
> ```powershell
> cd D:\git\cart-pole\training
> uv run yolo export model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.pt format=onnx imgsz=640
> ```
>
> Expected: `export success`, `weights\best.onnx` (~10 MB), input shape
> `(1, 1, 640, 640)` — single-channel baked into the artifact.
>
> Landed (2026-10-17): exactly that. Ultralytics auto-installed
> `onnxruntime` 1.31.0 + onnx/onnxslim into the uv env as a side effect
> (7 packages) — expected, do not repeat manually.

> ## Step 2 — validate the export (required before the Pi)
>
> Why: "export success" is a conversion receipt, not a test. Exporting
> traced the checkpoint's math into an ONNX graph and baked in the
> 640 input — a *new artifact* produced by a converter, and no image
> has ever gone through it. A converter can silently produce a file
> that loads fine, has the right input/output tensors, and still
> detects garbage (an op mis-converted, preprocessing mismatch). The
> only proof is running detections through it and scoring them — and
> the Pi can consume *only* this file, so without this step the first
> real execution of `best.onnx` would be on the rig, mid-session,
> far from a GPU and an easy fix.
>
> ```powershell
> uv run yolo detect val model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx imgsz=640 data=D:/git/cart-pole/datasets/generated/data.yaml project=D:/git/cart-pole/training/runs name=cp2-640-onnx-val
> ```
>
> The load-bearing fact is the pin: `onnxruntime-gpu==1.26.0` in
> `uv.lock` — the last CUDA 12.8 build, matching the CUDA runtime
> bundled inside torch (the machine's only CUDA libraries), and the
> newest with a Python 3.14 wheel. Torch is imported before the ONNX
> session loads, and its import registers the bundled DLL directory —
> that is where ONNX Runtime finds `cublasLt64_12.dll`, `cudnn64_9.dll`
> and the rest. Never upgrade onnxruntime-gpu past 1.26: newer builds
> need CUDA 13 libraries this machine does not have, and the step
> would silently drop to CPU.
>
> What running it does: loads `best.onnx` with **onnxruntime** on the
> `CUDAExecutionProvider` — not PyTorch; the same runtime family the
> Pi uses — pushes all 1,000 val frames (Set-11) through the exported
> graph, and scores every prediction against the ground-truth labels,
> printing the same per-class table as training (mAP50, mAP50-95,
> precision, recall). It changes nothing: the corpus and the `.onnx`
> are read-only; the only output is a report folder under
> `training\runs\`. ~21 seconds, ~5 ms inference per frame on the 3090.
> The success tell is in the header: `Using ONNX Runtime 1.26.0 with
> CUDAExecutionProvider` and no DLL warnings.
>
> **Pass:** every class lands ~0.99 and no class has collapsed. Landed
> (2026-10-17): all 0.994, Pivot 0.995, End 0.995. The export is
> proven; ship it to the Pi.
>
> The score is *higher* than the 0.971 printed at the end of training
> because the two were measured by different setups: 0.971 during
> training — big batches, at each epoch's end — and this check one
> image at a time, outside training. Same model, same 1,000 test
> images; the difference belongs to the measuring setup, not the model.
>
> **Fail:** the score collapses, one class dominates everything, or the
> boxes come out as garbage — the conversion broke the model. Stop;
> re-export and investigate here, never on the Pi.

> ## Step 3 — read the training results
>
> Why: every downstream decision — which model deploys, which retires
> to metrology, whether the new corpus actually improved transfer —
> reads from these tables, per class. An aggregate hides a failing
> marker.
>
> Files are in `training\runs\<name>\`:
>
> - `results.csv` — one row per epoch; the last row is the summary.
> - `weights\best.pt` — the checkpoint to keep (val-selected).
> - `confusion_matrix.png` — bright off-diagonal cells name confusions.
>
> Landed (2026-10-17):
>
> | Class | cp2-1440-base | cp2-640-base |
> |---|---|---|
> | all | 0.982 | 0.971 |
> | Left | 0.989 | 0.971 |
> | Right | 0.955 | 0.949 |
> | Pivot | 0.993 | 0.983 |
> | End | 0.990 | 0.982 |

> ## Step 4 — decide the model roles
>
> The decision rule: train at the scale you infer at. The 640 model
> recovers 90% of the cross-resolution gap (0.872 → 0.971) for ~3.7×
> less compute.
>
> - **`cp2-640-base` = deployment model** — runs on the Pi.
> - **`cp2-1440-base` = metrology model** — proposals and audits, on
>   the 3090. Per 06-REAL_LABELING.md: the instrument that proposes is
>   not the instrument being judged.

## Findings that carry forward

- **Right is the weakest class, geometrically.** Laggard at 1440
  (0.955), at 640 (0.949), and in the old corpus (0.962 at both
  scales): the 500 mm pole crosses Right's line of sight at swing
  extremes. No training regime fixes physics. Non-gating — Pivot and
  End, the control state, are the strongest classes everywhere.
- **End precision at 640 (0.949):** the pole tip draws false positives
  at small scale. Check `confusion_matrix.png` before deployment.
- **`epochs=500` with `patience=50`**: the 1440 run's best arrived at
  epoch 324 — a 200 cap would have truncated mid-improvement. Expect
  the 640 run to use all 500 (the drip keeps resetting patience).

## Reference — Pi timing (estimates until measured)

Compute per inference: ~6.5 GFLOPs at 640. Estimates for YOLO11n on a
Pi 5 CPU (~15–30 effective GFLOPS):

- 640: ~150–350 ms/frame ONNX fp32 (~3–7 Hz); ~60–120 ms/frame NCNN
  int8 (~8–15 Hz). The NCNN path is likely the difference between
  viable and not — measure both on the actual Pi.
- 1440: ~1.5–2.5 s/frame — unreachable, full stop.

---

## Appendix — the old corpus record (retired, deleted)

The old corpus (10,300 frames, 1440×1080, one effective viewpoint,
deleted 2026-10-18 together with both training runs) produced these
numbers. Compare, don't continue. The corpus is **not regenerable** —
the old generation sets no longer exist; what survives is the record
and the lesson: a high val score on a self-similar exam is a vanity
metric. The failure it produced is documented in 07-REFINING.md's
premise.

| Class | cp-1440-base | cp-640-base |
|---|---|---|
| all | 0.985 | 0.981 |
| Left | 0.994 | 0.985 |
| Right | 0.962 | 0.962 |
| Pivot | 0.994 | 0.989 |
| End | 0.990 | 0.988 |

- cp-1440-base: early stop at epoch 160 (best 110), 7.8 h, batch 13.
- Cross-resolution at 640: 0.901 (uniform tightness tax); 800: 0.946.
- cp-640-base: 200 epochs, 2.407 h, batch 70 — 95% of the gap
  recovered. Right at exactly 0.962 at both scales proved its
  weakness geometric.

Model retention: every run writes only to its own `runs\<name>\`
directory (append-only; reused names auto-increment). For insurance
on a final checkpoint, copy it out of the tree (5.7 MB each). The
only shared file is `yolo11n.pt` — read-only pretrained weights.
