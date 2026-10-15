# Audit — `training-plan.md` and `training-plan-noob.md`

Audited 2026-10-14 against the landed repo: every component the plans cite
read in source, every corpus number re-measured on disk, every serialized
scene value read from `SampleScene.unity`. Nothing was executed in Unity —
this is source inspection plus shell measurement, not a play-mode
validation.

## Verdict

The mechanisms are real; the numbers are not. Every behavior claim the
plans make about the pipeline's code checked out against the landed
sources. Every number attributed to "the landed corpus" describes a corpus
that is not on disk, and neither plan describes the scene the way it
actually is.

## What the landed corpus actually is

`unity/projects/cart-pole/Assets/Captures/cart-pole/`:

- 4 frames (`frame_000000`–`frame_000003`), JPEG q90; `frame_000000`
  decoded as 1440×1080. Sizes: 135,681 / 106,627 / 63,396 / 84,511 B,
  avg 97,554 B (95.3 KB).
- 4 label files, 15 boxes total, avg 142.5 B. `frame_000001` carries no
  class-1 (Right) box — the behind-camera drop rule, working.
- `markers.json` at the run root (0/376, 1/596, 2/920, 3/937).
- **No `wip~` directory exists.** Every sidecar size in the plans (8K
  render, crops, manifests, per-frame totals) is unmeasurable today.
- No `verification_stats.json` exists anywhere in the repo (repo-wide
  search). By the plans' own Phase 3 provenance gate, this corpus is
  refused as training data.
- Box widths from the landed labels: 0.016689–0.023155 normalized →
  24.0–33.3 px at 1440, 10.7–14.8 px at 640.

## Claims that are false or misattributed

| Claim (both docs unless noted) | Landed reality |
|---|---|
| Box widths "0.0165–0.027" from "the landed labels" (main §0, §4) | Landed labels measure 0.016689–0.023155. The cited range fits no artifact on disk |
| "landed pilot precedent: 27 confirmed / 1 dropped in 28" (main §1) / "the landed pilot: 27/1 in **28 frames**" (noob §1.4) | No stats file exists; the corpus holds 15 boxes across 4 frames. 28 was boxes from a gone 7-frame corpus; the noob doc relabeled boxes as frames |
| Frame avg 106.8 KB, label avg 147 B (main §0) | 95.3 KB and 142.5 B (4-frame corpus) |
| 8K render 1.13 MB, crops 8.5 KB, manifest 757 B, wip~ 1.17 MB/frame (main §0) | Unmeasurable — no `wip~` exists for any corpus on disk |
| "30k ≈ 3.2 GB + 35 GB — measured, not estimated" (main §2.5); "measured on the landed pilot corpus" (noob Part B) | Extrapolated from the gone 7-frame corpus; the landed 4-frame corpus cannot ground it |
| 8K pass "~36× the pixels" (main §2.4) | 7680×4320 / 1440×1080 = 21.3× |
| light-dim/bright: "read a landed manifest for a realistic lightYaw" (main §1, noob §1.1) | No manifest exists on disk (docs-off capture) — unexecutable as written |

Related provenance note: the AGENTS.md basis decision cites 66/80 vs
80/80 crops — an 80-crop corpus that is also not on disk. Same rule
applies: re-earn the numbers from the next run that writes a receipt.

## The scene is not the plans' Phase 0 state

Serialized values in `Assets/Scenes/SampleScene.unity`:

- `generateDebugImages: 0`, `generateDebugDocs: 0` — the scene is in the
  docs-off iteration state (the reason the corpus has no sidecar and no
  receipt). The noob doc's Phase 0.2 "must show on" is false against disk.
- `positionJitter: 0.5`, `jitterSeed: 0`, `states: 1`, `seed: 0` — not
  the pilot values either plan instructs (0.15 / 2001 / 300 / 1001).
- `robotBounds: {fileID: 0}` on the capture component — **unassigned**.
  A RobotBounds cube does exist in the scene, and the code's
  `FindAnyObjectByType` fallback covers it at runtime, but the noob doc's
  "assigned" claim is false as serialized.
- The randomizer's `desks` list holds 4 of 5 DeskSOs — Executive is not
  wired.
- Three forced situations already exist and are wired
  (`Assets/Situation-01/02/03.asset`), labels `frame_000008_state`,
  `frame_000012_state`, `frame_000025_state` — frame-reproductions mined
  from a longer earlier run that no longer exists. Both plans' Phase 1.1
  ("create them once") describes none of this.

Scene and corpus are mutually consistent: 3 forced + 1 random
(`states: 1`) = exactly the landed 4 frames, forced frames 0–2, random
frame 3.

## Doc-vs-doc deltas (the "do they agree" answer)

Substance agrees: class map, traps, run matrix, toggle semantics, disk
math, never-wipe rules, yaml, commands (reformatted to single-line
PowerShell), the Phase 5 bar. Four editorial deviations:

1. Split ratio rewritten: main says "~90/5/5 at scale"; the noob doc says
   "~90/10 pilot, ~95/5 at scale" and adds that the third bucket is
   played by Phase 5's real-camera set. Defensible (the main plan's own
   layout has only train/val), but a reader comparing both trips on it.
2. The `yolo11n-p2.pt` suggestion (main §4, the P2/stride-4 variant for
   the 640 run) is absent from the noob doc, whose §4.5 parenthetical
   "train the 640 variant once, record the answer, move on" is a
   garbled leftover — both runs were already trained two sections
   earlier.
3. The augmentation policy (`fliplr`, `degrees=0`/`perspective=0`,
   `copy_paste=0`, and especially "train on the frames as written — no
   re-encoding, no PNG copies") appears only in the main plan.
4. The noob doc's field-placement correction (`positionJitter`,
   `jitterSeed` live on the capture component, not the randomizer) is
   verified true — but it was published in the noob doc's own preamble
   instead of fixing `training-plan.md`, which still carries the error.

## Verified clean (source-inspected)

- **Rig slot order**, guid-resolved from `Rig.asset` to the MarkerSOs:
  0 = `markers/Left.asset` (0376), 1 = Right (0596), 2 = Pivot (0920),
  3 = End (0937) — matches the landed `markers.json`.
- The noob doc's headline correction is real: `positionJitter` (l. 43),
  `jitterSeed` (l. 45), `robotBounds` (l. 55) are `RigStateCapture`
  fields; `RigStateRandomizer` holds none.
- Defaults as documented: capture 960×540 code default (scene holds
  1440×1080); StateServer 960×540 default, port 5555, EncodeToJPG(90);
  randomizer states 100, interval 0.5, cartRange 0.45, poleSpan 3.0,
  light [2, 4], frontLightRange 90, skybox yaw [0, 360]; seed = 0 means
  fresh randomness (both randomizer and jitter).
- Verify semantics: never-wipe refusal fires only when confirmations
  equal zero with drops > 0; single-pass detection (exactly one id, equal
  to the class's tag); crop lookup wip~ first then root; in-memory basis
  detects the encoded JPEG bytes (byte-identical to a retained file);
  the rewrite happens only after the whole run is accounted.
- `markers.json` is written unconditionally at the run root;
  `classes.txt` follows the `wip~` directory's existence, not the docs
  gate; manifests and stats are docs-gated — all as documented.
- Run separation: a populated base gets a fresh `run_<timestamp>` child;
  the trigger is `frame_*.jpg` present or `markers.json` existing.
- 8K pass: 7680×4320, camera sharing the capture camera's transform and
  FOV, crops cut from projected quad corners with a ±20 px margin, tag
  zero-padded to 4 digits, no crop when any corner is behind the camera.
- The projection trap is honored in code: `WriteLabel` runs before the
  target restore, with the measured-skew rationale in the comment.
- Jitter: `FrontYaw()` keys off `Camera.main` (without it the sun yaw is
  uniform ±180°); the ride-along dolly to 50 m, the 0.05 m floor, the
  once-only no-bounds warning, and the per-frame jitter log line all
  match the docs.
- Forced situations: applied verbatim, forced first, false `use*` flags
  leave the scene pristine (deterministic only because forced states run
  first), `useCameraPosition` skips jitter, the manifest records what is
  actually in the frame.
- StateServer protocol matches `vision/gym_state_sender.py` exactly:
  one JSON line in (`id`, `joints[]`, optional `rig` payload), 4-byte
  little-endian length header + JPEG out; joint names
  `02-Cart.Slider.joint` / `04-Pole.Hinge.joint`.
- Menus as named: Register Desks (idempotent), Import HDRIs as Skyboxes
  (linear, clamp, 4096 cap, Panoramic), Capture Viewer, Run Report;
  Dataset Generator exists and both plans correctly exclude it.
- `MarkerManager`: class id derived from rig slot order (never authored),
  `Marker_<class>_<Label>` naming, anchor renderers/colliders disabled,
  the STEP-reimport rebind warning.
- Scene camera: physical (`m_projectionMatrixMode: 2`), focal 6 mm,
  sensor gate 4.968×3.726 mm, FOV 34.499° (the vertical FOV of that gate
  at 6 mm), tagged `MainCamera`.
- The stray `Assets/Cart-Pole/Left.asset` is a real blank MarkerSO (empty
  label, legacy `classId` field); the rig references
  `Assets/Cart-Pole/markers/Left.asset` — trap 7 confirmed.
- Pools: 5 DeskSOs (Writing, Standard, Panelend, Executive, Classic) and
  3 HDRI exr+material pairs (story_studio_02_4k, pav_studio_03_4k,
  neon_photostudio_4k).
- `training/` does not exist — the Phase 3 scripts are spec'd, not
  landed, exactly as the noob doc states; the PowerShell
  backslash-continuation warning is correct.

Nuance: Register Desks writes new DeskSOs to `Assets/Cart-Pole/Desks/`,
not the `Props/Desks/` folder where the landed five sit. The plans never
claim otherwise, so no defect — but that is where new assets will appear.

## Status of claims in this audit

- **Source-inspected (read, not executed):** every component listed
  above; `Rig.asset` + guid resolution; `SampleScene.unity` serialized
  values; the three Situation assets; menu implementations;
  `gym_state_sender.py`.
- **Measured (shell commands):** the corpus file list and byte sizes;
  label contents and box ranges; `frame_000000` dimensions; the absence
  of `wip~` and of any `verification_stats.json` repo-wide; the
  21.3× pixel-ratio arithmetic.
- **Not executed:** anything in Unity. Serialized scene values are file
  reads; live play-mode behavior is not attested by this audit.

## Fix list

Also found: the 8K camera's comment in `RigStateCapture.cs` claims "nearly
3x the pixel density" — the true ratios are 4–5.3× linear, 21.3× in area.
Not touched in this pass (code was out of scope).

### Agent-side — applied 2026-10-14

- [x] Corpus table re-attributed: landed 4-frame numbers for the root
  artifacts (97,554 B / 142.5 B, ~0.098 MB/frame), retired rows marked,
  no-manifest state stated — `training-plan.md` §0.
- [x] Box widths corrected at every copy (main §0, main §4.2, noob §4.3):
  0.016689–0.023155 → 24.0–33.3 px @1440, 10.7–14.8 px @640.
- [x] The 27/1-in-28 precedent re-attributed (retired 7-frame corpus, 28 =
  boxes not frames) — main Phase 1.4, noob §1.4.
- [x] Pilot-settings table split; `positionJitter`/`jitterSeed` moved to
  the capture component with scene deltas — main Phase 1.2, noob §1.2,
  noob preamble note rewritten.
- [x] lightYaw instruction no longer points at a nonexistent manifest;
  cites the landed Situation values (−23.7 / 75.5 / 33.8) — main
  Phase 1.1, noob §1.1.
- [x] Forced-situation reality documented (three frame-reproductions
  exist and are wired; count them) — main Phase 1.1, noob §1.1 + §1.3 +
  §1.4 (309), main §0 inventory.
- [x] Scene's true starting state documented (toggles off, jitter
  0.5/0, states 1/seed 0, robotBounds unassigned, 4-of-5 desks) — main
  Phase 0.3, noob Phase 0 intro + §0.2 + §0.4.
- [x] "36×" → 21.3×; disk math re-labeled estimate (3.2 → ~3 GB,
  re-measure clause) — main Phase 2.4/2.5, noob Part B + §2.5.
- [x] Landed `cart-pole` directory re-labeled: 4-frame docs-off smoke
  corpus, Phase 3 refuses it — main Phase 2.2.
- [x] Split ratios harmonized: 90/10 pilot, 95/5 scale, no third bucket
  — main Phase 3.4.
- [x] Noob §4.3 gains `yolo11n-p2.pt` and the no-re-encode rule;
  garbled §4.5 parenthetical fixed.
- [x] Appendix status sections updated in both docs; Appendix B marks
  the 80-crop trial as recorded evidence (AGENTS.md itself untouched).

### Yours — Unity editor

- [ ] Flip `generateDebugImages` + `generateDebugDocs` on (noob §0.2).
- [ ] `positionJitter` → 0.15, `jitterSeed` → 2001 (pilot values).
- [ ] Assign `robotBounds` on the capture component explicitly.
- [ ] Add `Desk (Executive)` to the randomizer's `desks` list.
- [ ] Randomizer pilot values: `states` 300, `seed` 1001.
- [ ] Keep-or-remove decision on the three frame-reproduction
  situations, and author the six starter situations (Phase 1.1).
- [ ] One-line comment fix in `RigStateCapture.cs` (the "3x" claim),
  when you next have the file open — code was out of this pass's scope.

### Re-earned by the next docs-on run (not fixable by editing)

- [ ] A `verification_stats.json` receipt — re-earns (or retires) the
  27/1 precedent and produces Phase 3's first acceptable corpus.
- [ ] `wip~` per-frame sizes (8K render, crops, manifests, total) —
  replaces the *retired* rows in the size table and grounds the disk
  math.
- [ ] Throughput (Phase 1 step 6) — wall-clock per frame, feeds Phase 2.
