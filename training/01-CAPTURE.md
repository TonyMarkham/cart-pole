# How the corpus was captured — the Unity side

The 11 batches behind this dataset were generated in the Unity editor
(`unity\projects\cart-pole`, scene `Assets\Scenes\SampleScene.unity`) by
two components on the `MarkerManager` object: **RigStateRandomizer**
(applies states) and **RigStateCapture** (renders and writes). This
file records how the landed corpus was produced and how to produce
more batches.

## Per-session settings (as landed)

`RigStateRandomizer`:

| Field | Pilot | Bulk runs |
|---|---|---|
| `states` | 300 | 1000 |
| `seed` | 1001 | 1002 … (+1 per run) |
| `interval` | 0.5 | 0.25 |
| `cartRange` | 0.45 | 0.45 |
| `poleSpan` | 3.0 | 3.0 |
| `randomizeDesks/Skybox/Light` | on | on |
| `forcedSituations` | empty (removed by decision) | empty |

`RigStateCapture`:

| Field | Value |
|---|---|
| `captureWidth/Height` | 1440 / 1080 (physical camera: gate 4.968x3.726 mm, 6 mm lens — the IMX296 rig match) |
| `jpegQuality` | 90 — the q90 contract: train and inference must share the same lossy statistics |
| `positionJitter` / `jitterSeed` | 0.15 / 2001 … (+1 per run, distinct from `seed`) |
| `outputDirectory` | was `Assets/Captures/cart-pole`; now `D:\git\cart-pole\datasets\cart-pole` — the corpus lives outside Unity |
| `generateDebugImages/Docs` | **off by decision** — the 8K crops are cut and detected in memory; nothing is retained, no `wip~` exists |

Every state is one capture: the randomizer applies cart slider, pole
hinge, a random desk, a random panoramic skybox (+ yaw 0-360), and a
random light intensity + yaw (front-hemisphere guaranteed via the
`MainCamera` tag); the capture then renders 1440x1080, projects the
marker quads into a YOLO label, and writes both. Total captures per
session = `states` (no forced situations).

## Console sequence (the session's heartbeat)

1. `plan: 0 forced + N random states, forced first`
2. one `[RigStateCapture] jitter: ...` line per frame
3. `done - 0 forced + N random states applied, forced first`
4. `[RigStateCapture] verification stats: N frames scanned, ... labels
   rewritten`

Line 4 is the in-memory verify: every marker's crop (cut from an 8K
in-memory render) must decode to exactly that class's tag or the box
is dropped. "labels rewritten" = the labels on disk now hold only
confirmed lines; "zero confirmations, labels kept as written" would be
a systemic failure — stop, don't scale. **A training batch must reach
line 4** — stopping Play early leaves a session with no verification
and no accounting.

## Run separation

A populated output base gets a fresh `run_<timestamp>` child per play
session — runs can never mix, and no directory preparation exists.
The pilot's 300 frames predate the child logic's use and sit flat at
the corpus root (`pilot-flat` in the build's naming).

## The 11 batches (as landed)

| Batch | Directory | Frames | Boxes |
|---|---|---|---|
| pilot | (flat at corpus root) | 300 | 1191 |
| 002542 | `run_20261015_002542` | 1000 | 3953 |
| 003702 | `run_20261015_003702` | 1000 | 3951 |
| 004306 | `run_20261015_004306` | 1000 | 3966 |
| 004826 | `run_20261015_004826` | 1000 | 3951 |
| 005344 | `run_20261015_005344` | 1000 | 3957 |
| 005932 | `run_20261015_005932` | 1000 | 3967 |
| 010454 | `run_20261015_010454` | 1000 | 3959 |
| 011011 | `run_20261015_011011` | 1000 | 3965 |
| 011536 | `run_20261015_011536` | 1000 | 3957 |
| 012055 | `run_20261015_012055` | 1000 | 3959 |

**Total: 10,300 frames, 40,776 boxes** (98.8% of 4-per-frame max; the
missing ~1% are markers whose quad was behind the camera — always the
cart's side markers at shallow angles, never more than one per frame).

Throughput: 0.25 s/frame — interval-bound, not render-bound (the 1440
render, the 8K pass, and both JPEG encodes fit inside the interval).

## Recipe for more batches

1. Bump the seed pair: `seed` +1 on the randomizer, `jitterSeed` +1 on
   the capture (any fresh nonzero pair works; +1 is the convention).
2. `states` 1000, `interval` 0.25 (or lower — it is pure pacing).
3. Press Play, walk away, wait for the `done` line, then the
   verification line. Stop Play.
4. The batch lands as a new `run_<timestamp>` child beside the others;
   rebuild the dataset (`build_dataset.py`) to include it.

Variety note: frames are cheap, the desk/HDRI pools are the real
budget (4 desks x 3 HDRIs as landed). Past ~10k frames, more pools
buy more real-world transfer than more frames.
