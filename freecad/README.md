# freecad/

The cart-pole's mechanical design: every part and assembly, drawn in
FreeCAD 1.1. It is the source of truth for the rig's shape and mass. The
Unity scenes and the MuJoCo model are built from the STEP file exported
from here (`EXPORT.md` has the export procedure).

## What is in here

| Folder or file | What it holds |
| --- | --- |
| `cad/` | The FreeCAD documents (`.FCStd`): one file per part (`cart.FCStd`, `washer-m3.FCStd`, …) and the assemblies (`asy-*.FCStd`) that place copies of those parts. `asy-cart-pole.FCStd` is the whole rig. |
| `materials/` | The material cards (`.FCMat`) the parts use that don't come with FreeCAD. See *Materials* below. |
| `step/` | The exported, mass-patched STEP (`asy-cart-pole.step`), the file Unity imports. |
| `tools/step-mass.py` | The patcher that writes each part's mass and density into the STEP, because FreeCAD's STEP export leaves them out. |
| `EXPORT.md` | How to assign materials, export the STEP and patch it, step by step. |

## Setting up FreeCAD (once per computer)

FreeCAD needs to be told where the repo's material cards are. Without
that, opening any assembly prints a long run of `Material not found` lines,
and the patcher can't find densities.

1. Install FreeCAD 1.1 and open it.
2. Open its Python console (**View → Panels → Python console**) and paste
   this line, with the path changed to **your** checkout's `freecad\materials`
   folder:

   ```python
   App.ParamGet("User parameter:BaseApp/Preferences/Mod/Material/Resources").SetString("CustomMaterialsDir", r"D:\git\cart-pole\freecad\materials")
   ```

3. Close FreeCAD and start it again. It reads its material folders only at
   startup.

The path must be absolute, the full path from the drive letter. FreeCAD
uses the setting exactly as written. A relative path would be read from
whatever folder FreeCAD was started in, usually its own install folder, not
from the repo. (Its material code, `Mod/Material/materialtools/cardutils.py`
in FreeCAD 1.1, does no other resolving.) If you move or re-clone the repo,
repeat step 2 with the new path.

The setting is stored in FreeCAD's own `user.cfg`, which FreeCAD rewrites
when it closes. So change it from inside FreeCAD, as above, never by editing
`user.cfg` while FreeCAD is running.

## Materials

A part file doesn't store its material, only the material's ID (a UUID,
for example `2135b6f2-3798-4aa2-92a8-216ad884abab`). When the file opens,
FreeCAD looks that ID up in the material cards it can see. If no card has
that ID, FreeCAD prints `Material not found` and the part has no density.
So the cards belong in the repo, beside the parts that use them.

The parts use six materials. Two come with FreeCAD; the other four are the
cards in `materials/`:

| Card | UUID | Density | Used by | Where the card lives |
| --- | --- | --- | --- | --- |
| AnyCubic Generic UV Resin | `a2666a77-67b3-416d-aab4-eb13cecdaf71` | 1200 kg/m³ | the 3D-printed parts: cart, base-idler, base-motor, the rod clamps, spacer, bearing cap, belt retainer, pole-target clamp and holder | `materials/` |
| Generic Steel | `2135b6f2-3798-4aa2-92a8-216ad884abab` | 7850 kg/m³ | the steel rods, screws, nuts, washers and ball bearings | `materials/` |
| Aluminium | `d5787d09-2f2f-46da-afdd-cfb5d570ff38` | 2700 kg/m³ | the pillow block and both GT2 pulleys | `materials/` |
| SPECIAL - Linear bearing | `7a8b8b13-6c91-4c64-8e89-b108afb2e98b` | 4278 kg/m³ | the 16 mm linear bearing | `materials/` |
| Default | `7f9fd73b-50c9-41d8-b7b2-575a030c1eeb` | 1 kg/m³ | the stepper, limit switch, `empty`, and the M3/M5 washer files | ships with FreeCAD (`Standard/Default.FCMat`) |
| Steel-Generic | `90bbd8ef-8623-4d78-b3bf-e0bdb9b74dd3` | 7900 kg/m³ | the M5 screw file | ships with FreeCAD (`Standard/Metal/Steel/Steel-Generic.FCMat`) |

Where these numbers come from: the four repo cards were lost from this
PC's FreeCAD library and rebuilt on 2026-11-10. The UUIDs are the ones the
part files point at. The names and densities are the ones `step-mass.py`
had written into `step/asy-cart-pole.step` from the original cards. The two
built-in rows are read from the cards in FreeCAD 1.1's install.

Rules for cards:

- **A card's file name is its material name.** FreeCAD 1.1 names a card
  after its file, not after the `Name:` line inside it, and the patcher
  writes that name into the STEP. `Generic Steel.FCMat` gives
  "Generic Steel"; `Generic-Steel.FCMat` would give "Generic-Steel".
- **Never change a card's UUID.** Every part using it would lose its
  material.
- **A new material goes into `materials/`**, as its own `.FCMat` file, and
  is committed with the parts that use it.

## Linked parts stay in `cad/`

An assembly doesn't copy its parts. It links to each part's file and stores
the path to that file, relative to the assembly. If a linked file isn't
where the path says, FreeCAD prints `Link not restored`, the part vanishes
from the assembly, and the STEP export leaves it out, mass included.

So every part an assembly uses must be a file in `cad/`, committed. Never
link a part straight from a download folder: copy it into `cad/` first,
then link the copy.

This is how the M2.5 washer and nut were lost. Both base assemblies linked
them from `D:\Downloads`, and the files later disappeared. On 2026-11-10 they
were rebuilt from their shapes in `step/asy-cart-pole.step` as
`cad/washer-m2.5.FCStd` and `cad/nut-m2.5x0.45.FCStd`, and the 12 links in
`asy-base-idler.FCStd` and `asy-base-motor.FCStd` were pointed at them.
