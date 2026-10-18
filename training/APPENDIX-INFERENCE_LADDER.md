# Appendix — Inference speed ladder (parked)

**Not on the execution path.** This is an appendix held in reserve:
a design for a deployment optimization that is built only if measured
Pi timing fails the control rate at the chosen resolution. The
walkthrough is docs 01–07; the estimates this appendix reacts to are
in `05-POST_TRAINING.md`.

## The gate

Build nothing in this doc until full-frame inference time has been
measured on the actual Pi 5. If the measured rate satisfies the
control loop, this design stays unbuilt. See 05-POST_TRAINING.md for
the estimates and the decision tree.

## The insight that makes it work: a 2-DOF linkage

The marker constellation has exactly two degrees of freedom:

- Cart position along the rail (1 DOF) — the synthetic corpus knows
  it twice over: the stepper commands are the motion prior, and the
  Left/Right/Pivot cluster is rigid (constant separations; one
  detection determines the others).
- Pole angle about the hinge (1 DOF) — End is constrained to a circle
  of radius L (pole length) around Pivot. Its reachable set is an
  arc, not a plane.

Calibration: **L in pixels** comes free from the corpus — the labels
contain thousands of Pivot→End distances; their mean is the rig's own
geometry, measured by the pipeline itself.

## The ladder — three windows, one set of weights

```
Rung 1  full frame (1456x1088)     heartbeat / re-anchor — rare
Rung 2  adaptive wide-short strip  the default frame
Rung 3  Pivot-centered 2L x 2L     contains ALL VISIBLE markers
```

- **Rung 2, the strip**: the constellation lives in a horizontal band.
  Crop full width × adaptive height, where the height bounds come
  from the past-positions array (running min/max of Pivot/End y +
  margin). Horizontal bounds: commanded stepper position ± margin.
  Calm system: ~600×300 (0.18 Mpx, ~9× less than the 1.58 Mpx full
  frame).
- **Rung 3, the anchored crop**: once Pivot is known, a 2L × 2L crop
  centered on Pivot contains the cart cluster and the entire End
  reachable circle. One small native-resolution inference covers
  every visible marker.
- **Rung 1 exists for recovery**: re-find the cluster, re-zero the
  estimators, re-tighten the strip. Rare by design.

Compute tally (Mpx, vs full frame 1.58): strip ~0.5, strip + anchored
crop ~0.5–0.7 — with markers kept at native 22–46 px instead of the
10–20 px a 640 letterbox shrinks them to.

## Design variants (pick when the gate fires)

- **A — predictive fovea**: last positions + velocity → predict →
  crop. Fastest, but inherits tracker burdens: per-marker state,
  lost-track protocol, reacquire path. The pole's 3-rad swing can
  outrun a stale prediction.
- **B — peripheral + fovea (two-pass, no prediction)**: cheap
  whole-frame pass (imgsz 320, ~5% of full-frame pixels) finds markers
  coarsely every frame; native-res crops refine around them. The
  peripheral pass is the reacquire mechanism — the track can never be
  lost. **Recommended default.**
- **C — strip + anchored crop**: the ladder as drawn.

## Mechanical details that decide whether it works

- **Feed crops at native scale, rectangular.** Letterboxing a
  1440×350 strip into a 640 square shrinks the markers back to 10 px
  and undoes the point. Ultralytics accepts rectangular
  `imgsz=[h, w]` (multiples of 32), e.g. `[320, 1440]`.
- **Scale mismatch caveat**: training letterboxed full frames, so
  markers were seen at 10–20 px; native-res strips present them at
  22–46 px. Bigger is usually easier, but box-tightness calibration
  shifts — validate the cropped mode on cropped val frames first, and
  fine-tune with band-crop augmentation if tightness drifts.
- **Containment is not visibility.** Crops guarantee where to look,
  never whether anything is there: the pole at swing extremes
  physically crosses the side markers' line of sight, and desks
  occlude. The observation builder must tolerate missing Left/Right
  (cart x comes from Pivot) — the baseline numbers already price this
  in (Right 0.962 on a non-gating class).
- **Free redundancy check**: the commanded stepper position versus the
  measured Left/Right midpoint should agree. Divergence means a
  mechanical slip or a detection hiccup — the loop learns which
  channel to trust, for free, because the geometry is rigid.

## How to validate before building

The ladder can be validated on the 3090 (cropped val frames) before
ever touching the Pi. If the cropped-mode accuracy holds, the ladder
is worth roughly an order of magnitude of inference time.
