# Post-training — resolution questions, export, and the path to the Pi

The 1440 base run is done (see `04-COMMANDS.md`). This file covers what
comes after: the resolution question the Pi forces, the export, and
the handoff to real-world frames.

## The landed baseline (cp-1440-base)

Early-stopped at epoch 160 (best at 110), 160 epochs in 7.8 h, batch 13
auto-fit, RTX 3090:

| Class | mAP50-95 | Note |
|---|---|---|
| all | 0.985 | P = 1.0, R = 1.0, mAP50 = 0.995 |
| Left | 0.994 | |
| Right | 0.962 | the laggard — seen most obliquely (also the most dropouts: 195) |
| Pivot | 0.994 | control-critical: box center = cart position |
| End | 0.990 | control-critical: Pivot→End vector = pole angle |

Pivot and End are the control state — cart x and pole angle derive
entirely from those two boxes; Left/Right are redundancy. The two
critical classes are the two best-scoring ones, so the Right weakness
does not gate anything.

Inference speed (3090): **4.0 ms per image at 1440** — ~250 fps of
headroom for the training loop.

## Question 1 — does the 1440-trained model survive 640 inference? (free, minutes)

YOLO11 is fully convolutional and anchor-free: the weights are not
welded to 1440, and the model runs at any input size. But inference at
640 letterboxes the 4:3 frame into a 640x640 square — markers shrink
from 22.5-46 px to ~10-20 px, and small objects are where detection
loses tightness.

```powershell
uv run yolo detect val model=D:/git/cart-pole/training/runs/cp-1440-base/weights/best.pt data=datasets/cart-pole/cart-pole.yaml imgsz=640
```

**Landed (2026-10-15):** mAP50 unchanged at 0.995 (every marker still
found, every time); the entire tax lands on tightness, and it is
**uniform** — every class drops ~0.07-0.11 with no class singled out.
That is the signature of a pure resolution effect, not a class
failure. The gap (0.901 vs 0.985) justified question 2.

## The resolution curve — three points, landed

The same val was run at 800 (snapped to ~800x608 — `imgsz` must be a
multiple of 32). Markers scale linearly with width:

| imgsz | all | Left | Right | Pivot | End |
|---|---|---|---|---|---|
| 1440 | 0.985 | 0.994 | 0.962 | 0.994 | 0.990 |
| 800 | 0.946 | 0.919 | 0.910 | 0.978 | 0.979 |
| 640 | 0.901 | 0.888 | 0.891 | 0.915 | 0.909 |
| **640-native** | **0.981** | **0.985** | **0.962** | **0.989** | **0.988** |

Findings:

- **The curve is smooth** — no knee; accuracy prices continuously at
  roughly +0.045 mAP50-95 per +160 px of width in this range.
- **The control-critical classes recover first.** At 800, Pivot and
  End sit within ~0.015 of their 1440 values, while the oblique cart
  markers carry the degradation — the baseline's pattern repeats at
  every scale. Since the observation vector derives from Pivot + End,
  the control state is nearly fully tight at 800.

## Question 2 — does training AT 640 fix what the curve shows? — ANSWERED

**Landed (2026-10-16):** `cp-640-base` completed all 200 epochs in
2.407 h (batch 70, no early stop — the drip kept resetting patience),
best.pt from the final validation:

| Class | mAP50-95 | vs 1440 |
|---|---|---|
| all | 0.981 | −0.004 |
| Left | 0.985 | −0.009 |
| Right | 0.962 | **0.000** |
| Pivot | 0.989 | −0.005 |
| End | 0.988 | −0.002 |

- The native-trained model recovered **95%** of the cross-resolution
  gap (0.901 → 0.981 of the 0.084 gap); the irreducible aggregate
  cost is **0.004** at **5x less Pi compute** than the 1440 ceiling
  model. 640 inference on the 3090: **1.1 ms**.
- **Right at exactly 0.962 — identical to 1440 — proves its weakness
  is geometric.** Two models, two resolutions, two training lengths,
  same number: the 500 mm pole physically crosses Right's line of
  sight at swing extremes. Resolution and training length are
  exonerated; the checkable prediction is that the missing-Right
  3-box frames in the corpus show the pole lying across the cart.
- **The control state is solved at 640**: Pivot 0.989 and End 0.988
  are within 0.005/0.002 of the 1440 ceiling, and the observation
  vector (cart x from Pivot, pole angle from Pivot→End) derives
  entirely from those two.

**Verdict: `cp-640-base` is the deployment model.** The 1440 run
retires to metrology duty (setup-time calibration, audits); 800 is
not justified by 0.004 of aggregate headroom.

## Model retention — experiments mature, they don't overwrite

Every training run writes only to its own `runs\<name>\` directory,
separated by the `name=` knob — so training `cp-640-base` cannot
touch `cp-1440-base`'s weights, results, or plots. Even reusing a
name is safe: Ultralytics auto-increments (`cp-1440-base2`) rather
than overwrite. The `runs\` tree is append-only by design.

That makes the post-training workflow **accretive**: each experiment
deposits a retained artifact, and the model zoo grows instead of
churning —

```
training\runs\
├── cp-1440-base\    weights\best.pt   the native-scale baseline
├── cp-640-base\     weights\best.pt   the deployment-scale candidate
└── cp-800-base\     (optional follow-up)
```

— and every later comparison reads its numbers from directories that
still exist. For extra insurance on a checkpoint you consider final,
copy it out of the tree (5.7 MB each):

```powershell
Copy-Item "D:\git\cart-pole\training\runs\cp-1440-base\weights\best.pt" "D:\git\cart-pole\training\cp-1440-base-best.pt"
```

The only file shared between runs is `yolo11n.pt` — the pretrained
starting weights, read-only.

## Export

```powershell
uv run yolo export model=D:/git/cart-pole/training/runs/cp-1440-base/weights/best.pt format=onnx imgsz=1440
```

(Use the winning resolution from questions 1-2.) Produces `best.onnx`.
`half=False` (default) is correct for the Pi's CPU runtime.

**The letterbox contract:** the ONNX input is the letterboxed square —
whatever runs on the Pi must preprocess exactly like Ultralytics
(scale to fit, pad). Validating the exported model against corpus
frames inside the uv env (needs `onnxruntime` — permission-gated
install) before it touches the Pi is the standing rule.

## Pi timing — estimates until measured on hardware

The 4 ms figure is a 3090 number and will not survive the trip. The
compute per inference (from the AutoBatch probes): **~6.5 GFLOPs at
640, ~34.5 GFLOPs at 1440** — the pixel ratio, exactly.

Rough, honestly-labeled estimates for YOLO11n on a Pi 5 CPU
(~15-30 effective GFLOPS depending on runtime and precision):

- 1440: ~1.5-2.5 s/frame (< 1 Hz) — **unreachable**, full stop
- 640: ~150-350 ms/frame ONNX fp32 (~3-7 Hz); **~60-120 ms/frame
  NCNN int8 (~8-15 Hz)** — the export format is not an optimization
  nicety here, it is likely the difference between viable and not
- 800: ~1.6x the 640 cost (~250-550 ms ONNX / ~100-190 ms NCNN)

The real numbers arrive only by running the export on the actual Pi
(Phase 5). Measure ONNX fp32 and NCNN int8 both — the NCNN path is
expected to be the one that makes the Pi viable.

## The real-frame set — the actual judge

Synthetic val mAP does not decide readiness; real frames do.

- Target: a few hundred real 1440x1080 frames from the rig across
  poses and lighting (~1,000-1,200 boxes hand-labeled).
- **Capture path discipline:** WebRTC is a preview channel (codec-
  processed, bitrate-adapted); the dataset frames must come from the
  raw camera path — the same frames the on-Pi inference consumes.
  Capture-through-codec != inference-through-raw.
- Lock AE/AWB on the camera: synthetic frames have fixed exposure;
  the real set should match rather than inject a gap.
- Score once: `uv run yolo detect val model=<exported> data=<real-val>.yaml imgsz=<winner>`
- Bar (adjustable): per-class AP50 >= 0.90, no class below 0.80.

Every real-world miss becomes either a hand-label for the test set or
a `ForcedSituationSO` for the next synthetic batch — the
failure-mining loop's input valve.

## Status of claims in this file

- **Measured (landed):** the 1440 baseline numbers, 7.8 h wall-clock,
  4.0 ms inference, batch 13, per-class table; the 640 cross-
  resolution val (0.901, uniform drop); the 800 cross-resolution val
  (0.946, control classes recovered); the 640-native run (200 epochs,
  2.407 h, best 0.981 aggregate — 95% of the gap recovered, per-class
  table landed, deployment model selected).
- **Not yet run:** any export, any Pi timing. The Pi numbers in this
  file are estimates from FLOP ratios, not measurements.
