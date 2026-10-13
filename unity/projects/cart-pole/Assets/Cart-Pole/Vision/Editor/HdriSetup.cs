using System.IO;

using UnityEditor;
using UnityEditor.SceneManagement;

using UnityEngine;
using UnityEngine.SceneManagement;

namespace CartPole.Vision.Editor
{
    // Turns the Polyhaven EXR HDRIs in Assets/Cart-Pole/Vision/HDRI into
    // panoramic skybox materials: linear float import, clamp wrapping, a
    // Skybox/Panoramic material each, and the active scene's skybox set
    // to the first one. The remaining materials are swapped in the
    // Lighting window or by re-running with a different first entry.
    internal static class HdriSetup
    {
        private const string Folder = "Assets/Cart-Pole/Vision/HDRI";

        [MenuItem("Cart-Pole/Import HDRIs as Skyboxes")]
        private static void Import()
        {
            var shader = Shader.Find("Skybox/Panoramic");
            if (shader == null)
            {
                Debug.LogError("[HdriSetup] Skybox/Panoramic shader not found");
                return;
            }
            string firstMatPath = null;
            foreach (var file in Directory.GetFiles(Folder, "*.exr"))
            {
                var path = file.Replace('\\', '/');
                var importer = (TextureImporter)AssetImporter.GetAtPath(path);
                importer.sRGBTexture = false;          // HDR data is linear
                importer.maxTextureSize = 4096;        // 4k source
                importer.wrapMode = TextureWrapMode.Clamp;
                importer.mipmapEnabled = false;
                importer.SaveAndReimport();

                var texture = AssetDatabase.LoadAssetAtPath<Texture2D>(path);
                var name = Path.GetFileNameWithoutExtension(path);
                var material = new Material(shader) { name = name };
                material.SetTexture("_MainTex", texture);
                var matPath = Path.ChangeExtension(path, ".mat");
                AssetDatabase.CreateAsset(material, matPath);
                firstMatPath = firstMatPath ?? matPath;
            }

            if (firstMatPath == null)
            {
                Debug.LogWarning($"[HdriSetup] no .exr files in {Folder}");
                return;
            }
            RenderSettings.skybox =
                AssetDatabase.LoadAssetAtPath<Material>(firstMatPath);
            EditorSceneManager.MarkSceneDirty(SceneManager.GetActiveScene());
            Debug.Log(
                "[HdriSetup] skybox materials created; scene skybox set to " +
                firstMatPath);
        }
    }
}
