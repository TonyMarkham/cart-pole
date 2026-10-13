using System;
using System.IO;
using System.Text.RegularExpressions;
using UnityEditor;
using UnityEngine;

namespace CartPole.Vision
{
    // Rasterizes the ArUco marker SVGs into seam-free PNG textures and
    // generates one URP/Lit material per marker. The SVGs import as UI
    // Toolkit VectorImage assets (no texture on import), and antialiased
    // rasterization of edge-adjacent rects leaves hairline seams between
    // cells - fatal for ArUco thresholding. The markers are plain
    // integer-grid rects, so they are rasterized procedurally instead:
    // exact pixels, point filtering, no seams.
    //
    // Drop the SVG assets into the list, then use the inspector context
    // menu to (re)generate. Existing materials are updated in place so
    // scene references survive regeneration.
    [CreateAssetMenu(menuName = "Cart-Pole/Marker Material Set", fileName = "MarkerMaterialSet")]
    public sealed class MarkerMaterialSet : ScriptableObject
    {
        [SerializeField] public UnityEngine.Object[] svgs = Array.Empty<UnityEngine.Object>();

        [SerializeField] string outputFolder = "Assets/Cart-Pole/markers";

        [SerializeField] int textureResolution = 256;

        [ContextMenu("Generate Materials")]
        public void Generate()
        {
            if (svgs == null || svgs.Length == 0)
            {
                Debug.LogWarning($"{name}: no SVGs assigned");
                return;
            }

            EnsureFolder(outputFolder, "Textures");
            EnsureFolder(outputFolder, "Materials");

            int generated = 0;
            foreach (var svg in svgs)
            {
                if (svg == null)
                    continue;

                string svgPath = AssetDatabase.GetAssetPath(svg);
                if (!svgPath.EndsWith(".svg", StringComparison.OrdinalIgnoreCase))
                {
                    Debug.LogWarning($"{name}: {svgPath} is not an SVG asset");
                    continue;
                }

                if (CreateTexture(svgPath) && CreateMaterial(svgPath))
                    generated++;
            }

            AssetDatabase.SaveAssets();
            Debug.Log($"{name}: generated {generated} marker textures/materials into {outputFolder}");
        }

        bool CreateTexture(string svgPath)
        {
            string name = Path.GetFileNameWithoutExtension(svgPath);
            string texturePath = $"{outputFolder}/Textures/{name}.png";

            Texture2D texture = RasterizeSvg(svgPath);
            if (texture == null)
            {
                Debug.LogWarning($"{name}: failed to rasterize {svgPath}");
                return false;
            }

            File.WriteAllBytes(texturePath, texture.EncodeToPNG());
            UnityEngine.Object.DestroyImmediate(texture);
            AssetDatabase.ImportAsset(texturePath);

            var importer = AssetImporter.GetAtPath(texturePath) as TextureImporter;
            if (importer != null)
            {
                importer.filterMode = FilterMode.Point;
                importer.wrapMode = TextureWrapMode.Clamp;
                importer.mipmapEnabled = false;
                importer.SaveAndReimport();
            }

            return true;
        }

        bool CreateMaterial(string svgPath)
        {
            string name = Path.GetFileNameWithoutExtension(svgPath);
            string texturePath = $"{outputFolder}/Textures/{name}.png";
            string materialPath = $"{outputFolder}/Materials/{name}.mat";

            var texture = AssetDatabase.LoadAssetAtPath<Texture2D>(texturePath);
            if (texture == null)
                return false;

            // Update in place so existing scene/material references survive
            var material = AssetDatabase.LoadAssetAtPath<Material>(materialPath);
            if (material == null)
            {
                material = new Material(Shader.Find("Universal Render Pipeline/Lit")) { name = name };
                AssetDatabase.CreateAsset(material, materialPath);
            }

            material.mainTexture = texture;
            EditorUtility.SetDirty(material);
            return true;
        }

        Texture2D RasterizeSvg(string svgPath)
        {
            string svg = File.ReadAllText(svgPath);

            var viewBoxMatch = Regex.Match(svg, @"viewBox=""([-\d.]+)\s+([-\d.]+)\s+([\d.]+)\s+([\d.]+)""");
            if (!viewBoxMatch.Success)
                return null;
            float viewX = float.Parse(viewBoxMatch.Groups[1].Value);
            float viewY = float.Parse(viewBoxMatch.Groups[2].Value);
            float viewW = float.Parse(viewBoxMatch.Groups[3].Value);
            float viewH = float.Parse(viewBoxMatch.Groups[4].Value);

            var rectMatches = Regex.Matches(
                svg,
                @"<rect\s+x=""([-\d.]+)""\s+y=""([-\d.]+)""\s+width=""([\d.]+)""\s+height=""([\d.]+)""\s+fill=""(#[0-9a-fA-F]{6})""");
            if (rectMatches.Count == 0)
                return null;

            var rects = new (float x, float y, float w, float h, Color32 color)[rectMatches.Count];
            for (int i = 0; i < rectMatches.Count; i++)
            {
                var m = rectMatches[i];
                ColorUtility.TryParseHtmlString(m.Groups[5].Value, out var color);
                rects[i] = (
                    float.Parse(m.Groups[1].Value),
                    float.Parse(m.Groups[2].Value),
                    float.Parse(m.Groups[3].Value),
                    float.Parse(m.Groups[4].Value),
                    color);
            }

            int size = textureResolution;
            var texture = new Texture2D(size, size, TextureFormat.RGBA32, false);
            var pixels = new Color32[size * size];
            float scaleX = size / viewW;
            float scaleY = size / viewH;

            // Paint rects in document order; sample at pixel centers so
            // shared integer edges resolve unambiguously to exactly one cell.
            for (int py = 0; py < size; py++)
            {
                // SVG y is top-down, Unity texture rows are bottom-up
                float svgY = viewY + viewH - (py + 0.5f) / scaleY;
                for (int px = 0; px < size; px++)
                {
                    float svgX = viewX + (px + 0.5f) / scaleX;

                    var color = new Color32(0, 0, 0, 0);
                    foreach (var (x, y, w, h, rectColor) in rects)
                    {
                        if (svgX >= x && svgX < x + w && svgY >= y && svgY < y + h)
                            color = rectColor;
                    }

                    pixels[py * size + px] = color;
                }
            }

            texture.SetPixels32(pixels);
            texture.Apply();
            return texture;
        }

        static void EnsureFolder(string folder, string name)
        {
            if (!AssetDatabase.IsValidFolder(folder))
            {
                Debug.LogWarning($"Output folder {folder} does not exist");
                return;
            }
            if (!AssetDatabase.IsValidFolder($"{folder}/{name}"))
                AssetDatabase.CreateFolder(folder, name);
        }
    }
}