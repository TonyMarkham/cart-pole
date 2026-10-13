# Synthetic Data Generation — Cart-Pole Marker Detection

How this repo generates a labeled object-detection training set for the
ArUco markers on the imported cart-pole assembly — with no manual
labeling. Every frame is born labeled: the marker quads are placed
programmatically from ground-truth rig bindings, and the labels are
derived from their projected corners, not annotated by hand.

## The big picture

```
RigSO (asset)                    scene
+ markers[4]          ->  MarkerManager (robot + rig refs)
+ cart/pole paths             |  applies marker quads under the
+ stepAsset (+ path)          |  assembly's anchors
                              v
                     RigStateRandomizer  --StateApplied-->  RigStateCapture
                     applies X random states                renders camera ->
                     as JSON through                        writes frame_NNNNNN.jpg
                     MarkerManager.ApplyState               + frame_NNNNNN.txt (YOLO)
                                                            + classes.txt
```

Review the output inside Unity: `Cart-Pole -> Capture Viewer`.

## Core concepts

**StepSiblingPath** (Greyhound.Step package) — the identity of a part:
the path of sibling indices from the assembly root to the GameObject
(`root child 4`, `its child 53`, ...). The STEP import rebuilds the same
hierarchy deterministically, so index paths survive renames where name
keys would not, and distinguish duplicate names for free.

**RigSO** (`Assets/Cart-Pole/Vision/RigSO.cs`) — the single binding
asset. It owns:

- `markers` — the four Marker SOs (End, Left, Pivot, Right)
- `cart`, `pole` — StepSiblingPaths of the two moving parts
- `stepAsset` (+ derived `stepAssetPath`) — the imported STEP assembly
  the paths index into

Everything is bound in its inspector: markers and the STEP asset are
object fields; the cart/pole parts are bound by *dragging the part from
the hierarchy into the drop zone* (writes the sibling path + STEP
source, with undo). A `Save` button writes the asset to disk.

**MarkerSO** (`MarkerSO.cs`) — one marker definition: class id, label,
prefab (quad at physical size), size, local offset/euler, and its anchor
StepSiblingPath. Created via right-click on a part ->
`GameObject/Add/Marker`, which also registers it in the rig.

**MarkerManager** (`MarkerManager.cs`) — the scene-side applier. Holds
`robot` (the scene instance of the assembly) and `rig`. `Apply`
(ContextMenu "Apply Markers", or `applyOnStart`) resolves each marker's
anchor through its sibling path, disables the anchor's
renderer/colliders, and instances the marker prefab under it.

## Scene setup (one time)

1. Drop the imported STEP assembly into the scene.
2. `MarkerManager` lives on **its own GameObject** in the scene — never
   on the assembly — with `rig` assigned and `robot` pointing at the
   assembly instance. Keeping it off the assembly means scene
   restructuring never moves, scales or duplicates it, and multiple
   assemblies each get their own manager. The `GameObject/Add/Marker`
   menu creates and wires this GameObject automatically (reusing an
   existing manager already bound to the same assembly).
3. `RigStateRandomizer` — the state driver (see below); assign its
   `markerManager` explicitly.
4. `RigStateCapture` — assign `randomizer` + `captureCamera` +
   `markerManager`, set `outputDirectory` (default
   `Assets/Captures/cart-pole`).
5. On the `StateServer` (gym loop only): assign `markerManager`.

## The state payload

`MarkerManager.ApplyState(string json)` parses:

```json
{"cartPosition": 0.1, "poleRotation": 0.3}
```

- `cartPosition` — meters along the rail: the **cart part's local +X**
- `poleRotation` — radians about the hinge: the **pole part's local -Z**
  through its own origin

These conventions are the assembly's own (the axes the STEP joint assets
author); the Rig alone locates the parts. Base poses are captured once
from the assembled state. There is also an overload
`ApplyState(float, float)` for callers that already have floats.

For gym training the `StateServer` (TCP, port 5555) accepts the same
state and captures the frame:

```json
{"id": 9, "rig": {"apply": true, "cartPosition": 0.1, "poleRotation": 0.3}}
```

and answers with a 4-byte little-endian length header + JPEG bytes.
Note: do not send `rig` and `joints` in the same message — JointDriver
and the rig pipeline would fight over the same transforms.

## Capturing

`RigStateRandomizer` applies `states` random states (uniform in
`+-cartRange` meters / `+-poleSpan` radians, defaults 0.45 m / 3 rad)
every `interval` seconds, pushing each through the **JSON** path so the
payload round-trip is exercised. `RigStateCapture` listens to the
`StateApplied` event and per state writes:

- `frame_NNNNNN.jpg` — JPEG by decision: the physical camera feeds the
  detector lossy JPEGs, so the synthetic set shares the same lossy
  statistics (quality default 90, matching the StateServer loop). No
  PNG mixing.
- `frame_NNNNNN.txt` — YOLO label: one line per visible marker,
  `class cx cy w h` normalized, **top-left origin** (Unity's viewport is
  bottom-up; the writer flips y). Each marker quad's four corners are
  projected through the capture camera (pinhole model — FOV/aspect, no
  lens distortion) and the label is the axis-aligned bounding box of the
  projected quad. Markers with any corner behind the camera are skipped;
  the rest clamp to the frame. Empty files are valid negatives.
- `classes.txt` — class names in id order, derived from the placed
  markers (`Marker_<class>_<Label>` naming), never hardcoded.

Marker resolution retries per capture: the first state can fire before
`MarkerManager.Start` has applied the quads, and an empty resolution is
never cached.

## Reviewing

`Cart-Pole -> Capture Viewer`: pages through a capture directory,
drawing each JPEG with its boxes overlaid, color-coded per class with
names from `classes.txt`. Missing/empty/off-frame labels are visible at
a glance. No external tooling.

## Gotchas

- **STEP reimports regenerate the imported objects** — a stale
  `stepAsset` reference reads as null. Resolution falls back to the
  stored `stepAssetPath` (which survives reimports); rebind the
  reference by re-dropping a part or re-assigning the field when you can.
- **The robot must be a scene instance** — a prefab asset has no live
  hierarchy to index into.
- **Don't drive the same parts twice** — JointDriver and ApplyState both
  position the cart/pole; run one at a time.
- **Marker quads are not parts** — drops named `Marker_*` are rejected
  by the rig inspector's drop zones.

## File map

| File | Role |
| --- | --- |
| `Assets/Cart-Pole/Vision/RigSO.cs` | the binding asset (markers, cart/pole paths, STEP asset) |
| `Assets/Cart-Pole/Vision/MarkerSO.cs` | one marker definition |
| `Assets/Cart-Pole/Vision/MarkerManager.cs` | applies markers; `ApplyState` positions cart/pole |
| `Assets/Cart-Pole/Vision/RigStateRandomizer.cs` | test/data driver: X random states as JSON |
| `Assets/Cart-Pole/Vision/RigStateCapture.cs` | JPEG + YOLO label + classes.txt writer |
| `Assets/Cart-Pole/Vision/StateServer.cs` | TCP JSON bridge (gym) |
| `Assets/Cart-Pole/Vision/JointDriver.cs` | legacy joint-based driver (qpos by joint name) |
| `Assets/Cart-Pole/Vision/Editor/RigSOEditor.cs` | rig inspector: drop zones, accordions, Save |
| `Assets/Cart-Pole/Vision/Editor/MarkerSOEditor.cs` | marker inspector: anchor accordion, rebind |
| `Assets/Cart-Pole/Vision/Editor/CaptureViewer.cs` | in-editor label review |
| `Assets/Cart-Pole/Vision/Editor/DatasetGeneratorWindow.cs` | older generator (JointDriver-based) |
