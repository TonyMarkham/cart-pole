# STEP export procedure

How to produce STEP files that carry correct mass and material density for
consumers (Unity `com.greyhound.step`, and any other OCCT-based importer).

FreeCAD 1.1 exports STEP geometry only: no material, no density, no mass.
The patcher in `tools/step-mass.py` injects an AP214 mass property and an
OCCT-canonical material/density property (identical to what
`STEPCAFControl_Writer` emits) directly into the exported STEP text.

## 1. Assign a material with a density (FreeCAD GUI)

FreeCAD 1.1 stores a part's material in its `ShapeMaterial` property, which
the patcher reads. The proper way to set it is FreeCAD's own **Material**
command, not the raw property:

1. Open the part's source document (`freecad/cad/<part>.FCStd`).
2. Select the **Body** (or the feature carrying the final geometry).
3. **Right-click the Body in the model tree → Select material.** This opens
   FreeCAD's Material task panel — it edits the selected object's
   `ShapeMaterial`.
4. Pick a material card whose **Density** is set (e.g. *AnyCubic Generic UV
   Resin*, 1.2e-06 kg/mm³) and confirm.

Editing the `ShapeMaterial` property directly in the property editor edits
the same data, but the Material command is the supported path.

For **assemblies**: set the material once in each part's **source document**
— the assembly's `App::Link` objects mirror the source part's material
automatically. Do not set materials on links or on the assembly container;
the container is skipped by the patcher (it has no own material and no own
geometry), and every part product gets its own mass and density, with every
instance of a part inheriting them.

## 2. Export the STEP (FreeCAD GUI)

1. Select the **Body** (or the **Assembly** container for assemblies).
2. File → Export… → file type **STEP with colors (*.step *.stp)**.
3. Save into `freecad/step/` with **exactly the same stem as the FCStd**:

    cad/rod-clamp-16mm.FCStd  →  step/rod-clamp-16mm.step

The name match is how the patcher finds the source model.

## 3. Patch mass and density (bash terminal, any directory)

```bash
freecad.cmd /home/tony/git/cart-pole/freecad/tools/step-mass.py -- \
    /home/tony/git/cart-pole/freecad/step/<part>.step
```

Notes:

- The bare `--` is required: without it freecadcmd consumes the script's
  options as its own.
- Paths are absolute above so the command works from any directory; the
  patcher resolves the FCStd relative to the STEP file (`../cad/<stem>.FCStd`).
- `--dry-run` prints volume, density, material and mass without writing.
- `--density '7800 kg/m^3'` overrides the material density for files whose
  source has none.
- `--fcstd cad/x.FCStd` points at a source whose name does not match.
- `--out other.step` writes elsewhere instead of in place.

The patcher is idempotent: re-running updates the existing values in place
and never duplicates entities. It refuses to write when the STEP geometry
volume differs from the source model by more than 1% (stale export —
re-export first).

## 4. Verify (optional)

```bash
grep -A1 "MEASURE_REPRESENTATION_ITEM" freecad/step/<part>.step
```

Expect a `MASS_MEASURE(<kg>)` line (mass in kilograms) and a
`RATIO_MEASURE(<g/cm3>)` line (density in grams per cubic centimetre).

## 5. Consume in Unity

The `com.greyhound.step` package reads the density through OCCT's XCAF
material table and computes mass, centre of mass and inertia from the exact
BRep. Requirements:

- The native host must be the build that reads the stored XCAF density
  directly (`step_mesh.cpp` `stored_density_g_per_cm3()`), i.e. the current
  `dist/package/.../Plugins/x86_64/libimporter_host.so`.
- The Unity project must reference the package as it does in
  `Packages/manifest.json` (`file:.../greyhound-snake/dist/package/...`).
- **Restart the Unity Editor completely after replacing any native `.so`**:
  Unity maps native libraries once per Editor process; re-importing without
  a process restart keeps calling the old binary.

Expected on the `Step Mass Properties` component:

| Field | Value |
| --- | --- |
| Used File Density | checked |
| Density G Per Cm 3 | the file's density, e.g. 1.2000000476837158 (float32 of 1.2) |
| Volume Mm 3 | the part volume in cubic millimetres |
| Mass Kilograms | volume × density × 1e-6 |
| Inertia Tensor | kg·m², principal moments about the centre of mass |

For assemblies every instance node carries its part's values; the assembly
root itself gets no mass component.

## Troubleshooting

| Message | Cause and fix |
| --- | --- |
| `no matching FCStd found` | STEP stem has no `cad/<stem>.FCStd`; rename or pass `--fcstd`. |
| `object '<x>' has no material density` | Do step 1, or pass `--density`. |
| `STEP geometry volume ... differs from source model` | The STEP is older than the model; re-export and re-run. |
| Unity still shows `Used File Density` unchecked or density 1 | The Editor is still running the old native library — fully quit Unity (process exit), reopen, delete the imported assets and re-import. |
| Density shows 0.0012 | A host binary built before the `stored_density_g_per_cm3` fix; rebuild with `just assemble-package` and restart the Editor. |

## Rules

- Never hand-edit a patched STEP file; re-run the patcher instead.
- Never treat a fresh FreeCAD export as final: exporting wipes everything
  the patcher injected, so step 3 always follows step 2.
- Keep the STEP stem and the FCStd stem identical.
