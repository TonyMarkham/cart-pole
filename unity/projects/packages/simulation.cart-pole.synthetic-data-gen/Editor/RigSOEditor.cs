using System.Collections.Generic;

using Greyhound.Step;

using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.SceneManagement;
using UnityEngine.UIElements;

namespace Simulation.CartPole.SyntheticDataGen.Editor
{
    // The rig asset's inspector: a bound UXML form plus the part drop
    // zones rendered in code. The rig binds three things — the four
    // Marker SOs of the rig (their default serialized array UI, since
    // assets need no drop affordance), and the StepSiblingPath
    // identities of the cart and pole parts (the same rename-robust
    // reference style the Marker SOs use for their anchors). An SO
    // cannot hold a scene reference, so the rig carries the marker
    // assets themselves; the scene-side MarkerManager consumes the rig.
    // A part binds by dropping it, which writes the sibling-index path
    // plus the STEP source the path indexes into (RigSO.m_StepAssetPath,
    // the direct analog of MarkerSO.stepAssetPath). The resolution and
    // path helpers replicate StepJointSetFactory's logic against public
    // Unity APIs; the factory is internal to the package. Rebinding
    // follows the drawer's affordance — a drop zone writing through the
    // serialized property so undo applies. Validation covers the cheap
    // field-level rules: an empty path and a path that no longer
    // resolves inside the source's instance. The base type is
    // qualified: the enclosing namespace's trailing ".Editor" segment
    // would otherwise bind the bare name to itself.
    [CustomEditor(typeof(RigSO))]
    internal sealed class RigSOEditor : UnityEditor.Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        private Label m_cartEmptyWarning;
        private Label m_cartUnresolvedWarning;
        private Label m_poleEmptyWarning;
        private Label m_poleUnresolvedWarning;
        private VisualElement m_cartSlot;
        private VisualElement m_poleSlot;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(RigSOEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(RigSOEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            m_cartEmptyWarning = root.Q<Label>("cart-empty-warning");
            m_cartUnresolvedWarning = root.Q<Label>("cart-unresolved-warning");
            m_poleEmptyWarning = root.Q<Label>("pole-empty-warning");
            m_poleUnresolvedWarning = root.Q<Label>("pole-unresolved-warning");
            m_cartSlot = root.Q<VisualElement>("cart-slot");
            m_poleSlot = root.Q<VisualElement>("pole-slot");

            RebuildPart("m_Cart", m_cartSlot);
            RebuildPart("m_Pole", m_poleSlot);
            UpdateForm();
            root.Q<Button>("save-button")?.RegisterCallback<ClickEvent>(_ => Save());
            // A hand-assigned STEP asset changes the resolution basis, so
            // the accordions rebuild with it and the stored path is
            // re-synced from the new reference (guarded against
            // recursion); other changes only need the warnings refreshed.
            root.RegisterCallback<SerializedPropertyChangeEvent>(change =>
            {
                if (change.changedProperty.propertyPath == "m_StepAsset")
                {
                    var reference = change.changedProperty.objectReferenceValue;
                    string path = reference != null
                        ? AssetDatabase.GetAssetPath(reference)
                        : null;
                    SerializedProperty stored =
                        serializedObject.FindProperty("m_StepAssetPath");
                    if (stored != null && stored.stringValue != path)
                    {
                        stored.stringValue = path;
                        serializedObject.ApplyModifiedProperties();
                    }
                    RebuildPart("m_Cart", m_cartSlot);
                    RebuildPart("m_Pole", m_poleSlot);
                }
                UpdateForm();
            });
            return root;
        }

        // The explicit persist: marks the asset dirty and writes it to
        // disk. Rebinds already dirty the asset through
        // ApplyModifiedProperties, but the import pipeline only picks an
        // SO up on an explicit save, so the button forces the flush.
        private void Save()
        {
            EditorUtility.SetDirty(target);
            AssetDatabase.SaveAssetIfDirty(target);
        }

        private void OnEnable()
        {
            Undo.undoRedoPerformed += OnUndoRedo;
        }

        private void OnDisable()
        {
            Undo.undoRedoPerformed -= OnUndoRedo;
        }

        // Undo of a rebind is not a serialized property change event, so
        // the drop zones and the warnings are rebuilt here; the bound
        // fields refresh themselves.
        private void OnUndoRedo()
        {
            serializedObject.Update();
            RebuildPart("m_Cart", m_cartSlot);
            RebuildPart("m_Pole", m_poleSlot);
            UpdateForm();
        }

        private void UpdateForm()
        {
            if (m_cartEmptyWarning == null)
            {
                return;
            }
            // The rig's fields are private (the reference style the
            // directive fixes), so reads go through the serialized
            // properties, not the target.
            serializedObject.Update();
            UpdatePartForm("m_Cart", m_cartEmptyWarning, m_cartUnresolvedWarning);
            UpdatePartForm("m_Pole", m_poleEmptyWarning, m_poleUnresolvedWarning);
        }

        private void UpdatePartForm(
            string propertyName, Label emptyWarning, Label unresolvedWarning)
        {
            SerializedProperty indices = serializedObject.FindProperty(propertyName)
                ?.FindPropertyRelative("indices");
            bool hasPath = indices != null && indices.arraySize > 0;
            emptyWarning.style.display =
                hasPath ? DisplayStyle.None : DisplayStyle.Flex;
            bool unresolved = false;
            if (hasPath)
            {
                Transform instanceRoot = ResolveInstanceRoot(StepAssetPath());
                if (instanceRoot != null)
                {
                    List<Transform> chain =
                        ResolveChain(instanceRoot, ReadPath(indices));
                    unresolved = chain.Count > 0 && chain[chain.Count - 1] == null;
                }
                else
                {
                    unresolved = true;
                }
            }
            unresolvedWarning.style.display =
                unresolved ? DisplayStyle.Flex : DisplayStyle.None;
        }
        // The per-part accordion the marker editor renders for its
        // anchor, rebuilt for the rig context: the live chain from the
        // stored source's instance root down the index path, trailing
        // nulls where the path no longer resolves, plus the creation-time
        // name and the drop zone that rebinds.
        private void RebuildPart(string propertyName, VisualElement slot)
        {
            if (slot == null)
            {
                return;
            }
            slot.Clear();
            serializedObject.Update();
            SerializedProperty partProperty = serializedObject.FindProperty(propertyName);
            StepSiblingPath stored = partProperty != null ? ReadPath(partProperty.FindPropertyRelative("indices")) : null;
            string createdName = partProperty != null
                ? partProperty.FindPropertyRelative("name").stringValue
                : null;
            string stepAssetPath = StepAssetPath();

            Transform root = ResolveInstanceRoot(stepAssetPath);
            List<Transform> chain = root != null && stored != null
                ? ResolveChain(root, stored)
                : new List<Transform>();
            Transform leaf = chain.Count > 0 ? chain[chain.Count - 1] : null;
            var foldout = new Foldout
            {
                text = leaf != null
                    ? leaf.name
                    : root != null ? "(unresolved)"
                    : !string.IsNullOrEmpty(stepAssetPath) ? "(assembly not in a scene)"
                    : "(no STEP source — assign the STEP asset above, then drop a part)",
                value = false,
            };

            var rows = new VisualElement();
            rows.AddToClassList("sibling-path-chain");
            for (int i = 0; i < chain.Count; i++)
            {
                Transform node = chain[i];
                Label row;
                if (node == null)
                {
                    row = new Label("(missing)");
                }
                else if (i == 0)
                {
                    row = new Label(node.name);
                }
                else
                {
                    row = new Label($"{node.name}  [{stored.indices[i - 1]}]");
                }
                row.AddToClassList(i == chain.Count - 1
                    ? "sibling-path-chain__row--target"
                    : "sibling-path-chain__row");
                rows.Add(row);
            }
            var createdRow = new Label($"created for: {createdName ?? "(none)"}");
            createdRow.AddToClassList("sibling-path-chain__row");
            rows.Add(createdRow);
            foldout.Add(rows);
            slot.Add(foldout);

            var dropZone = new VisualElement();
            dropZone.AddToClassList("sibling-path-dropzone");
            dropZone.Add(new Label($"drop the {propertyName} robot-assembly part to rebind"));
            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = TryGetValidDrop(stepAssetPath, out _, out _, out _)
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                if (!TryGetValidDrop(stepAssetPath, out GameObject dropped, out GameObject instanceRoot, out string stepPath))
                {
                    return;
                }
                WritePart(propertyName, dropped, instanceRoot, stepPath);
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
            });
            slot.Add(dropZone);
        }

        // A private field cannot be read off the target from the editor
        // assembly, so the path is materialized from its serialized
        // property.
        private static StepSiblingPath ReadPath(SerializedProperty indices)
        {
            var path = new StepSiblingPath();
            if (indices == null)
            {
                return path;
            }
            path.indices = new int[indices.arraySize];
            for (int i = 0; i < indices.arraySize; i++)
            {
                path.indices[i] = indices.GetArrayElementAtIndex(i).intValue;
            }
            return path;
        }

        // The rebind write: through the serialized properties so undo
        // applies, then rebuilds the accordion and the warnings. Both
        // paths index into the same assembly, so the drop also binds the
        // shared STEP source — as the imported asset reference, not a
        // path string.
        private void WritePart(
            string propertyName, GameObject part, GameObject instanceRoot, string stepPath)
        {
            serializedObject.Update();
            SerializedProperty partProperty = serializedObject.FindProperty(propertyName);
            SerializedProperty indices = partProperty.FindPropertyRelative("indices");
            int[] path = PathIndices(part, instanceRoot);
            indices.arraySize = path.Length;
            for (int i = 0; i < path.Length; i++)
            {
                indices.GetArrayElementAtIndex(i).intValue = path[i];
            }
            partProperty.FindPropertyRelative("name").stringValue = part.name;
            serializedObject.FindProperty("m_StepAsset").objectReferenceValue =
                AssetDatabase.LoadAssetAtPath<GameObject>(stepPath);
            serializedObject.FindProperty("m_StepAssetPath").stringValue = stepPath;
            serializedObject.ApplyModifiedProperties();
            RebuildPart(propertyName, propertyName == "m_Cart" ? m_cartSlot : m_poleSlot);
            UpdateForm();
        }

        // The resolution basis: the path derived from the stepAsset
        // reference while it is live, else the stored path — the
        // reference breaks when a STEP reimport regenerates the imported
        // objects, while the path string survives.
        private string StepAssetPath()
        {
            SerializedProperty stepAsset = serializedObject.FindProperty("m_StepAsset");
            if (stepAsset != null && stepAsset.objectReferenceValue != null)
            {
                return AssetDatabase.GetAssetPath(stepAsset.objectReferenceValue);
            }
            SerializedProperty stored = serializedObject.FindProperty("m_StepAssetPath");
            return stored != null ? stored.stringValue : null;
        }

        // A drop is valid when the dragged object is a part of the bound
        // STEP assembly and not the assembly root itself — the source it
        // resolves to must be the asset in the rig's stepAsset field, so
        // a part of any other assembly (or a drop before the asset is
        // assigned) is rejected. The generated marker quads sit in the
        // hierarchy beside the anchors and are not parts.
        private static bool TryGetValidDrop(
            string expectedStepPath,
            out GameObject dropped, out GameObject instanceRoot, out string stepPath)
        {
            dropped = null;
            instanceRoot = null;
            stepPath = null;
            if (string.IsNullOrEmpty(expectedStepPath))
            {
                return false;
            }
            foreach (Object reference in DragAndDrop.objectReferences)
            {
                if (reference is GameObject candidate &&
                    !candidate.name.StartsWith("Marker_") &&
                    TryResolveStepSource(candidate, out GameObject root, out string source) &&
                    candidate != root &&
                    source == expectedStepPath)
                {
                    dropped = candidate;
                    instanceRoot = root;
                    stepPath = source;
                    return true;
                }
            }
            return false;
        }

        // The chain of nodes a path walks through, assembly root first
        // and the anchored part last; trailing nulls where the path no
        // longer resolves (a shifted index path). Mirrors the package's
        // ResolveChain.
        private static List<Transform> ResolveChain(Transform root, StepSiblingPath path)
        {
            var chain = new List<Transform> { root };
            Transform node = root;
            foreach (int index in path.indices)
            {
                node = node != null && index >= 0 && index < node.childCount
                    ? node.GetChild(index)
                    : null;
                chain.Add(node);
            }
            return chain;
        }

        // The instance of the stored STEP source: the open prefab stage
        // when it stages that asset, else the scene hierarchy. Mirrors
        // the package's ResolveInstanceRoot.
        private static Transform ResolveInstanceRoot(string stepAssetPath)
        {
            if (string.IsNullOrEmpty(stepAssetPath))
            {
                return null;
            }
            PrefabStage stage = PrefabStageUtility.GetCurrentPrefabStage();
            if (stage != null && stage.assetPath == stepAssetPath && stage.prefabContentsRoot != null)
            {
                return stage.prefabContentsRoot.transform;
            }
            for (int i = 0; i < SceneManager.sceneCount; i++)
            {
                foreach (GameObject sceneRoot in SceneManager.GetSceneAt(i).GetRootGameObjects())
                {
                    Transform found = FindInstanceRoot(sceneRoot.transform, stepAssetPath);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
            return null;
        }

        private static Transform FindInstanceRoot(Transform node, string stepAssetPath)
        {
            GameObject instanceRoot = PrefabUtility.GetNearestPrefabInstanceRoot(node.gameObject);
            if (instanceRoot != null &&
                PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(instanceRoot) == stepAssetPath)
            {
                return instanceRoot.transform;
            }
            foreach (Transform child in node)
            {
                Transform found = FindInstanceRoot(child, stepAssetPath);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }

        // Mirrors StepJointSetFactory.TryResolveStepSource: prefab-stage
        // edits resolve against the stage root, scene instances against
        // the nearest prefab instance root; both must carry a STEP
        // extension.
        private static bool TryResolveStepSource(
            GameObject part, out GameObject instanceRoot, out string stepPath)
        {
            instanceRoot = null;
            stepPath = null;

            PrefabStage stage = PrefabStageUtility.GetCurrentPrefabStage();
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

            GameObject sourceRoot = PrefabUtility.GetNearestPrefabInstanceRoot(part);
            if (sourceRoot == null)
            {
                return false;
            }
            string path = PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(sourceRoot);
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
        private static int[] PathIndices(GameObject part, GameObject instanceRoot)
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
