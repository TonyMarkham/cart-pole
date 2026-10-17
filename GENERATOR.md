# The generator directives

One page of binding directives for the new synthetic-data generator.
Decided 2026-10-16 in the audit session; written down so the session
record is not the only home. The generator behavior must be written
against this page. Where it conflicts with `DATASETS.md`, this page
wins — `DATASETS.md` remains a design doc that dissolves into the
numbered docs once implemented.

## Home and ownership

- The generator lives in the local Unity package
  `simulation.cart-pole.synthetic-data-gen`
  (`unity/projects/packages/`). It references **nothing** in
  `Assets/Cart-Pole/Vision` — the legacy scripts are reading material
  only and will be deleted. Inspiration may be drawn from them; code
  may not.
- The scene wires one `SyntheticDataGenerator` GO: rig, capture
  camera, directional light, `RobotBounds`, `m_KeepBoundsInFrustrum`,
  and the list of `GenerationSetSO`s. The rig's `RigSO.Apply` is the
  only marker-application path.
- Once generation runs through the new workflow, the old
  `MarkerManager` GO is deleted — no transition period, no dual-run
  support. Its file (still `namespace CartPole.Vision` inside the
  package) dies with it unless the migrated state-drive mechanism
  needs its scene-root resolution.

## The rig (`RigSO`)

- Class ids are the rig's marker slot order: 0 = Left (tag 376),
  1 = Right (596), 2 = Pivot (920), 3 = End (937). `markers.json` is
  the machine-readable map, written from the rig's marker order —
  never retyped anywhere.
- **State drive:** the legacy `ApplyState` mechanism, migrated. Base
  poses captured once from the assembled (state = 0) rig; the cart is
  translated along the rail's **local +X** (meters), the pole rotated
  about the hinge's **local -Z** (radians). Payload shape
  `{cartPosition, poleRotation}` — the gym bridge's wire contract
  stays exactly this.
- **State draw bounds are authored on the rig:** cart position range
  ±0.45 m (the slider joint's own limit — local), pole rotation range
  ±`Mathf.PI` (the full half turn the hinge allows — local).

## The set (`GenerationSetSO`)

- **All set ranges are ABSOLUTE world values**, not offsets from the
  authored scene poses. (Only the rig's cart range is local — it is
  the rail's axis.)
- **Camera per draw:** world euler (pitch, yaw, 0) at world
  (lateral, height, z). Yaw ±5°, pitch ±5°, height 0.125–0.375
  (world Y), lateral ±0.125 (world X). After the random transforms,
  the camera **dollies along its Z until the RobotBounds is fully
  inside the frustum** (`m_KeepBoundsInFrustrum`).
- **Max Z (3.5 m, authored on all 13 sets):** the cap on the
  frustum-compensation dolly. A draw whose compensation exceeds it is
  **not captured and does not count toward `States`** — the generator
  redraws, so every set still yields its full state count.
- **Light:** absolute yaw [-60°, 60°], absolute pitch [5°, 85°],
  intensity [2, 4]. **No front-light guarantee** — the yaw range
  replaced it, deliberately.
- **Desks:** the 4-desk pool. The Executive desk stays excluded by the
  standing 2026-10-14 decision. Skyboxes: the 3-HDRI pool, yaw
  [0°, 360°] through the material's `_Rotation`.
- **No forced situations.** Dropped — the production corpus' 11,000
  random captures stand in. `ForcedSituationSO` and its editor leave
  with the legacy workflow; no run manifest carries a situation label.

## Run trigger and pacing

- **Play mode only.** Editor-mode generation is rejected — it makes
  crap data. Play mode "worked well"; it stays the trigger.
- **Synchronous per state:** render → 8K pass → marker discovery →
  label write all complete before the next state is drawn. No
  interval, no coroutine pause — the legacy hard wait between states
  is gone.
- One run iterates `m_Sets` in list order; each set writes its own
  `run_<set>_<timestamp>` folder (the set's SO name, for record
  keeping) under that set's `OutputDirectory`.

## The format (per run folder)

Exactly three things, all committed — **there is no `wip~`
directory.** The instruments are in-memory steps that leave no files.

```
run_<timestamp>/
  markers.json            the class map (rig slot order)
  images/
    frame_NNNNNN.jpg      1456x1088 greyscale JPEG, quality 90
  labels/
    frame_NNNNNN.txt      class x_center y_center w h, normalized,
                          top-left origin (y flipped from Unity)
```

- **Greyscale everywhere — no colour anywhere.** The capture frames,
  the 8K verification renders, and the per-marker crops are all
  greyscale; the generator has no colour render path. JPEG q90 and
  greyscale are hardcoded format constants, not SO fields.
- **Trees:** `datasets/generated/` (synthetic) and `datasets/real/`
  (real, evaluation-only). These supersede `DATASETS.md`'s
  `cart-pole` / `cart-pole-real` names. Each run lands where its
  set's `OutputDirectory` points; the sim/real split stays enforced
  by path.

## Verification (never a toggle)

- Verification is **unconditional** — no authored flag exists or will
  exist. Every frame passes the gate.
- The gate: an 8K greyscale render (7680x4320 via 4x supersampling —
  the empirically-confirmed sweet spot where OpenCV's ArUco reliably
  found all markers), per-marker crops cut in memory, detected, then
  discarded. A label line is written **only when its class's crop
  decodes to exactly that class's tag**. A missing crop means no YOLO
  data for that marker in that frame.
- **Never-wipe:** the writer refuses a zero-confirmation rewrite; no
  code path may empty a label file. The rule lives in the generator's
  code, not in a setting.

## Validation sets

- `Set-A` (5 states) and `Set-B` (6 states) are the only sets wired
  into the generator's `m_Sets` — validation runs produce 11 frames
  total, cheap enough to watch end to end, gate included.
- `Set-01`…`Set-11` are the production configuration (1000 states
  each, seeds 1001…1111, `datasets/generated/`), wired in after
  validation. All 13 sets author `Max Z = 3.5`.
