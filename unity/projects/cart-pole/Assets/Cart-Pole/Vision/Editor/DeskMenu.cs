using System.IO;

using UnityEditor;
using UnityEngine;

namespace CartPole.Vision.Editor
{
    // Registers the imported desk FBXs as DeskSO assets: one asset per
    // model under Assets/Cart-Pole/Desks, with default placement
    // modifiers (origin position, zero rotation, unit scale) that the
    // author tunes in the inspector. Idempotent - existing DeskSOs are
    // left untouched, so re-running only picks up newly imported models.
    internal static class DeskMenu
    {
        [MenuItem("Cart-Pole/Register Desks")]
        private static void RegisterDesks()
        {
            const string sourceFolder = "Assets/Cart-Pole/Props/Desks";
            const string outputFolder = "Assets/Cart-Pole/Desks";
            if (!AssetDatabase.IsValidFolder(outputFolder))
            {
                AssetDatabase.CreateFolder("Assets/Cart-Pole", "Desks");
            }
            var created = 0;
            foreach (var file in Directory.GetFiles(sourceFolder, "*.fbx"))
            {
                var path = file.Replace('\\', '/');
                var name = Path.GetFileNameWithoutExtension(path);
                var outPath = $"{outputFolder}/{name}.asset";
                if (AssetDatabase.LoadAssetAtPath<DeskSO>(outPath) != null)
                {
                    continue;
                }
                var prefab = AssetDatabase.LoadAssetAtPath<GameObject>(path);
                if (prefab == null)
                {
                    Debug.LogWarning($"[DeskMenu] no importable model at {path}");
                    continue;
                }
                var desk = ScriptableObject.CreateInstance<DeskSO>();
                desk.label = name;
                desk.prefab = prefab;
                AssetDatabase.CreateAsset(desk, outPath);
                created++;
            }
            AssetDatabase.SaveAssets();
            Debug.Log($"[DeskMenu] {created} DeskSO assets registered in {outputFolder}");
        }
    }
}
