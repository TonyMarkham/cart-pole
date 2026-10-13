# Skybox Capture & Unity Import Guide

## Part 1 — Sourcing Skyboxes

### Verified sources (researched)

| Source | License | Content | URL |
|---|---|---|---|
| **PolyHaven** | CC0 (no attribution) | HDRIs, equirect, up to 8K+ | https://polyhaven.com/hdris |
| **ambientCG** | CC0 | HDRIs (LatLong), free downloads | https://ambientcg.com/hdris |
| **NASA SVS** | Public domain | Space/planet/starfield visualizations, frames in .exr/.tiff, images in .jpg/.png/.tiff | https://svs.gsfc.nasa.gov/ |
| **Unity Asset Store** | Per-asset EULA | Dedicated skybox category, many free packs, drop-in cubemats/materials | https://assetstore.unity.com/categories/tools/essentials/skybox |
| **HDRI Haven** | CC0 | Merged into PolyHaven — same library, redirects there | https://polyhaven.com/hdris |

Notes:
- For a **space/sci-fi scene**, NASA SVS is the strongest free source (public domain, real astronomy data, no attribution needed).
- For **general environments**, PolyHaven is the strongest (true HDR, CC0, horizon-centered equirect).
- Asset Store packs come as ready-made cubemap/skybox materials — zero prep work, but check each pack's license.

### From PolyHaven (best source for HDR)

1. Go to https://polyhaven.com/hdris — all assets are free (CC0, no attribution required).
2. Download an HDR as **`.hdr` or `.exr`** (choose 4K–8K resolution).
3. Skip Part 2 entirely — PolyHaven HDRIs are already correct LatLong 2:1, high dynamic range, and horizon-centered. Import straight into Unity (Part 3, step 1) with **sRGB off** and assign to a Skybox/Panoramic material.

### From Panini (Steam)

Panini saves skyboxes as equirectangular panoramas inside `.skybox` files, which are actually **ZIP archives**.

1. Locate them under the install folder, e.g.:
   ```
   Steam\steamapps\common\Panini\panini\...\saves\...\.skybox
   ```
2. Copy the `.skybox` file, rename the extension to `.zip`, and extract the image inside (usually JPG/PNG, 4K–8K wide).

### Capturing your own (photo mode)

1. Use Panini's photo mode / free camera; hide HUD and UI.
2. Force an **equirectangular 360 capture** (some builds need a `--panoramic` launch option or a 360-screenshot mod).
3. Capture at the highest resolution available — ideally **8192×4096**.
4. Keep camera **roll at 0°** or the skybox will be tilted when imported.

## Part 2 — Prepping the Image

Unity's Skybox/Panoramic shader expects a **LatLong (equirectangular, 2:1)** image.

1. Crop to **exactly 2:1** (e.g. 6144×3072). The horizon must sit in the vertical center.
2. **Mirror horizontally** if the capture reads reversed (buildings/text flipped) — equirect captures are sometimes stored mirrored.
3. Export as **JPG quality 90+** (no alpha needed). For realistic ambient/reflection lighting, prefer **HDR** (`.hdr`/`.exr`) — LDR JPGs work but ambient comes out dimmer.

### Sky-only captures (void below the horizon)

Either:
- Mirror the top half downward to fake a ground, or
- Leave it and place a real ground plane mesh in the Unity scene below the horizon.

## Part 3 — Importing into Unity

1. Copy the image into `Assets/Skyboxes/`.
2. Select the texture → Inspector:
   - **Texture Shape:** leave as 2D (the Skybox/Panoramic shader handles the mapping), or set **Sphere** for GPU sampling convenience.
   - **sRGB:** on for LDR, off for HDR linear sources.
3. Create a material, shader **Skybox/Panoramic**, assign the texture. Set **Mapping = Equirectangular**.
4. **Window → Rendering → Lighting → Environment** → assign the material as **Skybox Material**.
5. In the same window, set **Static Lighting Sky** to the same material and regenerate lighting so the sky drives ambient.
6. Mipmaps: disable them only if you need crisp stars and the texture is small; otherwise leave default filtering on.

## Part 4 — Swapping skyboxes at runtime

```csharp
using UnityEngine;

public class SkyboxSwapper : MonoBehaviour
{
    [SerializeField] private Material[] skyboxes;
    private int index;

    public void Next()
    {
        index = (index + 1) % skyboxes.Length;
        RenderSettings.skybox = skyboxes[index];
        DynamicGI.UpdateEnvironment();
    }
}
```

Make sure the materials are referenced by a scene object or included in a build (Resources/Addressables) — unreferenced materials get stripped from builds.

## Part 5 — Verify it worked

- **Editor check:** camera `Clear Flags = Skybox` and `RenderSettings.skybox != null` on the active camera.
- **In game:** rotate the camera 360° — the sky should stay fixed relative to the world (no swimming).
- **Seam check:** a color jump at the wrap point means the image's left/right edges don't match. Fix by offsetting the image 50% horizontally in an image editor and blending the seam, then re-import.

---

## Project-specific notes

- Ambient in this project measured 0.21–0.26 and there's no reflection probe — an **HDR skybox** will noticeably brighten the scene once set as **Static Lighting Sky** and lighting is regenerated.
- One-click fix option: an editor script that assigns the material, sets Static Lighting Sky, and bakes — ask when ready.
