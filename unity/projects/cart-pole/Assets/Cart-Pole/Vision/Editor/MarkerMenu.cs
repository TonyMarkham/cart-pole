using System.Collections.Generic;
using System.IO;

using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace CartPole.Vision.Editor
{
    // Hierarchy right-click / GameObject menu entry for marker authoring,
    // mirroring the joint flow exactly: select the anchor part, run the
    // command; the STEP source is resolved, the sibling path is built from
    // the instance root (StepSiblingPath, same reference the Joint SOs
    // serialize) and a MarkerSO is created. Unity 6.7's rewritten Hierarchy
    // frames-and-renames after menu invocations, so the work is deferred one
    // editor tick (Defer).
    //
    // The resolution helpers replicate StepJointSetFactory's logic against
    // public Unity APIs; the factory itself is internal to the package.
    internal static class MarkerMenu
    {
        private static void AddMarkerInternal(GameObject part)
        {
            if (!TryResolveStepSource(part, out var instanceRoot, out var stepPath))
            {
                Debug.LogError($"{part.name}: not a STEP-imported part");
                return;
            }
            // The manager is its own GameObject in the scene - not a
            // component of the assembly - so scene restructuring never
            // moves it with the assembly. An existing manager bound to
            // this assembly is reused; otherwise a dedicated GameObject
            // is created and the robot reference is wired to the assembly
            // instance through its serialized property (the field is
            // private).
            var manager = FindOrCreateManager(instanceRoot, stepPath);

            // The rig owns the marker bindings: reuse the one bound to the
            // manager, else the existing rig for this assembly, else create
            // it; the manager always ends up pointing at it.
            var rig = manager.rig;
            if (rig == null)
            {
                rig = FindOrCreateRig(stepPath);
                manager.rig = rig;
                EditorUtility.SetDirty(manager);
            }

            // No class id is assigned here: a marker's class is its slot
            // in the rig's marker list, derived when the markers are
            // applied.

            if (!AssetDatabase.IsValidFolder("Assets/Cart-Pole/Markers"))
            {
                if (!AssetDatabase.IsValidFolder("Assets/Cart-Pole"))
                {
                    AssetDatabase.CreateFolder("Assets", "Cart-Pole");
                }
                AssetDatabase.CreateFolder("Assets/Cart-Pole", "Markers");
            }
            var assetPath = $"Assets/Cart-Pole/Markers/{part.name}.asset";
            var marker = ScriptableObject.CreateInstance<Simulation.CartPole.SyntheticDataGen.MarkerSO>();
            marker.label = part.name;
            marker.anchor.indices = PathIndices(part.transform, instanceRoot.transform);
            marker.anchor.name = part.name;
            marker.stepAssetPath = stepPath;
            AssetDatabase.CreateAsset(marker, assetPath);

            // The markers array is private on the rig (the binding lives on
            // the SO), so the write goes through its serialized object: the
            // first empty slot, else a grown array.
            var rigSerialized = new SerializedObject(rig);
            var markers = rigSerialized.FindProperty("m_Markers");
            if (markers.arraySize == 0)
            {
                markers.arraySize = 4;
            }
            var slot = -1;
            for (var i = 0; i < markers.arraySize; i++)
            {
                if (markers.GetArrayElementAtIndex(i).objectReferenceValue == null)
                {
                    slot = i;
                    break;
                }
            }
            if (slot < 0)
            {
                markers.arraySize++;
                slot = markers.arraySize - 1;
            }
            markers.GetArrayElementAtIndex(slot).objectReferenceValue = marker;
            rigSerialized.ApplyModifiedProperties();
            EditorUtility.SetDirty(rig);

            Selection.activeObject = marker;
            EditorGUIUtility.PingObject(marker);
        }

        // The MarkerManager for a STEP assembly: the existing manager
        // whose rig binds this assembly, else a fresh one on a dedicated
        // "MarkerManager" GameObject. The robot reference is written
        // through the serialized property because the field is private.
        private static CartPole.Vision.MarkerManager FindOrCreateManager(
            GameObject instanceRoot, string stepPath)
        {
            var source = AssetDatabase.LoadAssetAtPath<GameObject>(stepPath);
            foreach (var manager in Object.FindObjectsByType<CartPole.Vision.MarkerManager>())
            {
                if (manager.rig != null && manager.rig.StepAsset == source)
                {
                    return manager;
                }
            }
            var go = new GameObject("MarkerManager");
            Undo.RegisterCreatedObjectUndo(go, "Create Marker Manager");
            var created = go.AddComponent<CartPole.Vision.MarkerManager>();
            var serialized = new SerializedObject(created);
            serialized.FindProperty("robot").objectReferenceValue =
                instanceRoot.gameObject;
            serialized.ApplyModifiedPropertiesWithoutUndo();
            return created;
        }

        // The rig asset for a STEP assembly: the existing rig whose
        // m_StepAsset is the imported assembly asset, else a fresh one
        // created beside the marker assets. The m_StepAsset field is
        // private on the SO, so the reference is written through the
        // serialized object.
        private static Simulation.CartPole.SyntheticDataGen.RigSO FindOrCreateRig(string stepPath)
        {
            var source = AssetDatabase.LoadAssetAtPath<GameObject>(stepPath);
            foreach (var guid in AssetDatabase.FindAssets("t:RigSO"))
            {
                var candidate = AssetDatabase.LoadAssetAtPath<Simulation.CartPole.SyntheticDataGen.RigSO>(
                    AssetDatabase.GUIDToAssetPath(guid));
                if (candidate != null && candidate.StepAsset == source)
                {
                    return candidate;
                }
            }
            if (!AssetDatabase.IsValidFolder("Assets/Cart-Pole"))
            {
                AssetDatabase.CreateFolder("Assets", "Cart-Pole");
            }
            var fileName = Path.GetFileNameWithoutExtension(stepPath);
            var assetPath = AssetDatabase.GenerateUniqueAssetPath(
                $"Assets/Cart-Pole/Rig_{fileName}.asset");
            var rig = ScriptableObject.CreateInstance<Simulation.CartPole.SyntheticDataGen.RigSO>();
            var serialized = new SerializedObject(rig);
            serialized.FindProperty("m_StepAsset").objectReferenceValue = source;
            serialized.ApplyModifiedPropertiesWithoutUndo();
            AssetDatabase.CreateAsset(rig, assetPath);
            return rig;
        }

        // Mirrors StepJointSetFactory.TryResolveStepSource: prefab-stage
        // edits resolve against the stage root, scene instances against the
        // nearest prefab instance root; both must carry a STEP extension.
        private static bool TryResolveStepSource(
            GameObject part, out GameObject instanceRoot, out string stepPath)
        {
            instanceRoot = null;
            stepPath = null;

            var stage = PrefabStageUtility.GetCurrentPrefabStage();
            if (stage != null && stage.prefabContentsRoot != null)
            {
                if (!IsSelfOrDescendant(stage.prefabContentsRoot.transform, part.transform))
                {
                    return false;
                }
                instanceRoot = stage.prefabContentsRoot;
                stepPath = stage.assetPath;
                return HasStepExtension(stepPath);
            }

            var sourceRoot = PrefabUtility.GetNearestPrefabInstanceRoot(part);
            if (sourceRoot == null)
            {
                return false;
            }
            var path = PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(sourceRoot);
            if (!HasStepExtension(path))
            {
                return false;
            }
            instanceRoot = sourceRoot;
            stepPath = path;
            return true;
        }

        private static bool IsSelfOrDescendant(Transform parent, Transform node)
        {
            for (var t = node; t != null; t = t.parent)
            {
                if (t == parent)
                {
                    return true;
                }
            }
            return false;
        }

        private static bool HasStepExtension(string path)
        {
            var lower = path.ToLowerInvariant();
            return lower.EndsWith(".step") || lower.EndsWith(".stp");
        }

        // Sibling indices from the instance root down to the part.
        private static int[] PathIndices(Transform part, Transform instanceRoot)
        {
            var reversed = new List<int>();
            var node = part.transform;
            while (node != null && node.gameObject != instanceRoot)
            {
                reversed.Add(node.GetSiblingIndex());
                node = node.parent;
            }
            reversed.Reverse();
            return reversed.ToArray();
        }
    }
}
