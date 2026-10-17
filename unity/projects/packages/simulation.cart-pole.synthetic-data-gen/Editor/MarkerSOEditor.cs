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
    // The marker asset's inspector: a bound UXML form plus the anchor
    // accordion rendered in code. The marker is self-sufficient — anchor
    // path plus the STEP source it indexes into (MarkerSO.stepAssetPath,
    // the direct analog of the joint set's stepAssetPath) — so the
    // accordion resolves exactly the way the package resolves a joint's
    // body path: against the stored source's instance, prefab stage
    // first, then the open scenes. The manager that consumes markers is
    // a plain list; it plays no part here. The resolution and path
    // helpers replicate StepJointSetFactory's logic against public Unity
    // APIs; the factory is internal to the package. Rebinding follows
    // the drawer's affordance — a drop zone writing through the
    // serialized property so undo applies; a sibling-index path has no
    // scene-widget analog (the joint editor's scene tools pick its axis,
    // not its body path). Validation covers the cheap field-level rules:
    // an empty anchor path, an indexed path without a STEP source, and a
    // path that no longer resolves inside the source's instance.
    // The base type is qualified: the enclosing namespace's trailing
    // ".Editor" segment would otherwise bind the bare name to itself.
    [CustomEditor(typeof(MarkerSO))]
    internal sealed class MarkerSOEditor : UnityEditor.Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        private VisualElement m_anchorSlot;
        private Label m_anchorEmptyWarning;
        private Label m_anchorSourceWarning;
        private Label m_anchorUnresolvedWarning;
        private Label m_sizeWarning;
        private Image m_prefabPreview;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(MarkerSOEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label($"[{GetType().Namespace}::{nameof(MarkerSOEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            // The arUcoId field is delayed: it commits on focus lost or
            // Enter, not on every keystroke - per-keystroke commits fire
            // OnValidate (and its validation warning) on every character.
            var arUcoField = root.Q<PropertyField>("arUcoId-field");
            if (arUcoField != null)
            {
                arUcoField.schedule.Execute(() =>
                {
                    var textField = arUcoField.Q<TextField>();
                    if (textField != null)
                    {
                        textField.isDelayed = true;
                    }
                }).ExecuteLater(1);
            }

            root.Bind(serializedObject);

            m_anchorEmptyWarning = root.Q<Label>("anchor-empty-warning");
            m_anchorSourceWarning = root.Q<Label>("anchor-source-warning");
            m_anchorUnresolvedWarning = root.Q<Label>("anchor-unresolved-warning");
            m_sizeWarning = root.Q<Label>("size-warning");
            m_anchorSlot = root.Q<VisualElement>("anchor-slot");

            m_prefabPreview = new Image { scaleMode = ScaleMode.ScaleToFit };
            m_prefabPreview.AddToClassList("texture-preview");
            root.Q<VisualElement>("prefab-slot")?.Add(m_prefabPreview);

            RebuildAnchor();
            UpdateForm();
            root.Q<Button>("save-button")?.RegisterCallback<ClickEvent>(_ => Save());
            root.RegisterCallback<SerializedPropertyChangeEvent>(_ => UpdateForm());
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
        // the accordion and the warnings are rebuilt here; the bound
        // fields refresh themselves.
        private void OnUndoRedo()
        {
            serializedObject.Update();
            RebuildAnchor();
            UpdateForm();
        }

        private void UpdateForm()
        {
            var marker = (MarkerSO)target;
            if (marker == null || m_anchorEmptyWarning == null)
            {
                return;
            }
            StepSiblingPath anchor = marker.anchor;
            bool hasPath = anchor?.indices is { Length: > 0 };
            bool hasSource = !string.IsNullOrEmpty(marker.stepAssetPath);
            m_anchorEmptyWarning.style.display =
                hasPath ? DisplayStyle.None : DisplayStyle.Flex;
            // An indexed path without a STEP source cannot be resolved —
            // a legacy asset authored before the source was stored;
            // rebind to attach it.
            m_anchorSourceWarning.style.display =
                hasPath && !hasSource ? DisplayStyle.Flex : DisplayStyle.None;
            bool unresolved = false;
            if (hasPath && hasSource)
            {
                Transform instanceRoot = ResolveInstanceRoot(marker.stepAssetPath);
                if (instanceRoot != null)
                {
                    List<Transform> chain = ResolveChain(instanceRoot, anchor);
                    unresolved = chain.Count > 0 && chain[chain.Count - 1] == null;
                }
            }
            m_anchorUnresolvedWarning.style.display =
                unresolved ? DisplayStyle.Flex : DisplayStyle.None;
            if (m_sizeWarning != null)
            {
                m_sizeWarning.style.display =
                    marker.size > 0f ? DisplayStyle.None : DisplayStyle.Flex;
            }
            if (m_prefabPreview != null)
            {
                UpdatePrefabPreview(marker.prefab);
            }
        }

        // Unity's async prefab thumbnail comes back as a blank grey box, so
        // preview the marker texture directly: read it off the prefab's
        // renderer material (prefab assets can be read without
        // instantiation).
        private void UpdatePrefabPreview(GameObject prefab)
        {
            if (prefab == null)
            {
                m_prefabPreview.image = null;
                m_prefabPreview.style.display = DisplayStyle.None;
                return;
            }

            m_prefabPreview.style.display = DisplayStyle.Flex;
            var renderer = prefab.GetComponentInChildren<MeshRenderer>();
            m_prefabPreview.image = renderer != null && renderer.sharedMaterial != null
                ? renderer.sharedMaterial.mainTexture
                : null;
        }

        // The accordion the package's drawer renders for joints, rebuilt
        // for the marker context: the live chain from the stored source's
        // instance root down the index path, trailing nulls where the
        // path no longer resolves, plus the creation-time name and the
        // drop zone that rebinds.
        private void RebuildAnchor()
        {
            if (m_anchorSlot == null)
            {
                return;
            }
            m_anchorSlot.Clear();
            var marker = (MarkerSO)target;
            StepSiblingPath stored = marker != null ? marker.anchor : null;
            string stepAssetPath = marker != null ? marker.stepAssetPath : null;

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
                    : "(no STEP source — drop a part to rebind)",
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
            var createdRow = new Label($"created for: {stored?.name ?? "(none)"}");
            createdRow.AddToClassList("sibling-path-chain__row");
            rows.Add(createdRow);
            foldout.Add(rows);
            m_anchorSlot.Add(foldout);

            var dropZone = new VisualElement();
            dropZone.AddToClassList("sibling-path-dropzone");
            dropZone.Add(new Label("drop a robot-assembly part to rebind"));
            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = TryGetValidDrop(out _, out _, out _)
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                if (!TryGetValidDrop(out GameObject dropped, out GameObject instanceRoot, out string stepPath))
                {
                    return;
                }
                WriteAnchor(dropped, instanceRoot, stepPath);
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
            });
            m_anchorSlot.Add(dropZone);
        }

        // A drop is valid when the dragged object is a part of a
        // STEP-imported assembly and not the assembly root itself — the
        // same rule the package's drawer applies to a joint's body path.
        // The drop both binds the anchor and names the assembly the path
        // indexes into. The generated marker quads sit in the hierarchy
        // beside the anchors and are not parts.
        private static bool TryGetValidDrop(
            out GameObject dropped, out GameObject instanceRoot, out string stepPath)
        {
            dropped = null;
            instanceRoot = null;
            stepPath = null;
            foreach (Object reference in DragAndDrop.objectReferences)
            {
                if (reference is GameObject candidate &&
                    !candidate.name.StartsWith("Marker_") &&
                    TryResolveStepSource(candidate, out GameObject root, out string source) &&
                    candidate != root)
                {
                    dropped = candidate;
                    instanceRoot = root;
                    stepPath = source;
                    return true;
                }
            }
            return false;
        }

        // The rebind write: through the serialized properties so undo
        // applies, then rebuilds the accordion and the warnings — a raw
        // field write would not refresh the form.
        private void WriteAnchor(GameObject part, GameObject instanceRoot, string stepPath)
        {
            serializedObject.Update();
            SerializedProperty anchor = serializedObject.FindProperty("anchor");
            SerializedProperty indices = anchor.FindPropertyRelative("indices");
            int[] path = PathIndices(part, instanceRoot);
            indices.arraySize = path.Length;
            for (int i = 0; i < path.Length; i++)
            {
                indices.GetArrayElementAtIndex(i).intValue = path[i];
            }
            anchor.FindPropertyRelative("name").stringValue = part.name;
            serializedObject.FindProperty("stepAssetPath").stringValue = stepPath;
            serializedObject.ApplyModifiedProperties();
            RebuildAnchor();
            UpdateForm();
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
