# Inference speed ladder — the crop/prediction idea (parked, gated on Pi timing)

Everything here is a **post-training deployment optimization**. It is
gated on a measurement that does not exist yet: full-frame inference
time on the actual Pi 5. If the measured rate satisfies the control
loop, none of this should be built — it is complexity held in reserve,
not a step in the pipeline.

## The insight that makes it work: a 2-DOF linkage

The marker constellation has exactly **two degrees of freedom**:

- cart position along the rail (1 DOF) — and the synthetic RPi knows
  it twice over: the stepper commands *are* the motion prior, and the
  Left/Right/Pivot cluster is rigid (constant separations; one
  detection determines the others).
- pole angle about the hinge (1 DOF) — End is constrained to a circle
  of radius L (pole length) around Pivot. Its reachable set is an arc,
  not a plane.

So the camera is not tracking four independent objects; it is tracking
a rigid 2-DOF linkage whose half-state is published by the actuators.

Calibration: **L in pixels** comes free from the corpus — the labels
contain thousands of Pivot→End distances; their mean is the rig's own
geometry, measured by the pipeline itself.

## The ladder — three windows, one set of weights

```
Rung 1  full frame (1440x1080)     heartbeat / re-anchor — rare
Rung 2  adaptive wide-short strip  the default frame
Rung 3  Pivot-centered 2L x 2L     contains ALL VISIBLE markers
```

- **Rung 2, the strip**: the constellation lives in a horizontal band
  (rail + pole sweep). Crop full width x adaptive height, where the
  height bounds come from the **past-positions array** (running
  min/max of Pivot/End y + margin) — the crop breathes with the
  dynamics. Horizontal bounds: commanded stepper position +/- margin.
  Calm system: ~600x300 (0.18 Mpx, ~11x less than the 2.07 Mpx full
  frame). Wide swing: expands toward the full band.
- **Rung 3, the anchored crop**: once Pivot is known, a 2L x 2L crop
  centered on Pivot contains the cart cluster (it sits at the center)
  and the entire End reachable circle. One small native-resolution
  inference covers every visible marker.
- **Rung 1 exists for recovery**: re-find the cluster, re-zero the
  estimators, re-tighten the strip. Rare by design.

Compute tally (Mpx, vs full frame 2.07): strip ~0.5, strip + anchored
crop ~0.5-0.7, with markers kept at **native 22-46 px** — the scale
the model is strongest at, versus the 10-20 px a 640 letterbox
shrinks them to. Done right, foveation is not just faster than
full-frame; it is more accurate than full-frame at any rate the Pi
could afford.

## Design variants (pick when the gate fires)

- **A — predictive fovea**: last positions + velocity -> predict ->
  crop. Fastest (one small inference) but inherits tracker burdens:
  per-marker state estimation, lost-track protocol, reacquire path.
  The pole's 3-rad swing can outrun a stale prediction.
- **B — peripheral + fovea (two-pass, no prediction)**: cheap
  whole-frame pass (imgsz 320, ~5% of full-frame pixels) finds
  markers coarsely every frame; native-res crops refine around them.
  The peripheral pass IS the reacquire mechanism — the track can
  never be lost. Recommended default.
- **C — strip + anchored crop**: the ladder as drawn. Strip locates
  Pivot; geometry does the rest.

## Mechanical details that decide whether it works

- **Feed crops at native scale, rectangular.** Letterboxing a
  1440x350 strip into a 640 square shrinks the markers back to 10 px
  and undoes the point. Ultralytics accepts rectangular
  `imgsz=[h, w]` (multiples of 32), e.g. `[320, 1440]` — compute is
  the crop's area, not a square's.
- **Scale mismatch caveat**: training letterboxed full frames, so
  markers were seen at 10-20 px; native-res strips present them at
  22-46 px. Bigger is usually easier than smaller, but box-tightness
  calibration shifts — validate the cropped mode on cropped val
  frames first, and fine-tune with band-crop augmentation if
  tightness drifts.
- **Containment is not visibility.** Crops guarantee where to look,
  never whether anything is there: the 500 mm pole at swing extremes
  physically crosses the side markers' line of sight, and desks
  occlude. The observation builder must tolerate missing Left/Right
  (cart x comes from Pivot) — which the baseline numbers already
  price in (Right 0.962 on a non-gating class). The dropout
  hypothesis to check in the corpus: Left/Right dropouts cluster in
  frames where the pole lies across the cart.
- **Free redundancy check**: the commanded stepper position versus
  the measured Left/Right midpoint should agree. Divergence means
  either a mechanical slip or a detection hiccup — the loop learns
  which channel to trust, for free, because the geometry is rigid.

## Gate

Build only if measured Pi timing fails the control rate at the
chosen resolution (see `05-POST_TRAINING.md` for the estimates and
the decision tree). The ladder is worth roughly an order of
magnitude — and it can be validated on the 3090 (cropped val)
before ever touching the Pi.
