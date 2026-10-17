using System.IO;

using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Simulation.CartPole.SyntheticDataGen.Editor
{
    // The generation set asset's inspector: a bound UXML form plus the
    // output-directory drop zone rendered in code. An Explorer/Finder
    // drag carries no object references - only paths - so the zone
    // accepts a drag whose paths contain an existing directory and
    // writes the first one through the serialized property so undo
    // applies. The base type is qualified: the enclosing namespace's
    // trailing ".Editor" segment would otherwise bind the bare name to
    // itself.
    [CustomEditor(typeof(GenerationSetSO))]
    internal sealed class GenerationSetSOEditor : UnityEditor.Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

        private Label m_outputEmptyWarning;
        private Label m_desksEmptyWarning;
        private Label m_skyboxesEmptyWarning;
        private VisualElement m_outputDirectorySlot;

        public override VisualElement CreateInspectorGUI()
        {
            if (m_VisualTreeAsset == null)
            {
                return new Label(
                    $"[{GetType().Namespace}::{nameof(GenerationSetSOEditor)}]: VisualTreeAsset Not Set");
            }
            VisualElement root = m_VisualTreeAsset.CloneTree();

            if (m_StyleSheet == null)
            {
                return new Label(
                    $"[{GetType().Namespace}::{nameof(GenerationSetSOEditor)}]: StyleSheet Not Set");
            }
            root.styleSheets.Add(m_StyleSheet);

            root.Bind(serializedObject);

            m_outputEmptyWarning = root.Q<Label>("output-empty-warning");
            m_desksEmptyWarning = root.Q<Label>("desks-empty-warning");
            m_skyboxesEmptyWarning = root.Q<Label>("skyboxes-empty-warning");
            m_outputDirectorySlot = root.Q<VisualElement>("output-directory-slot");

            RebuildDropZone();
            UpdateForm();
            root.Q<Button>("save-button")?.RegisterCallback<ClickEvent>(_ => Save());
            root.RegisterCallback<SerializedPropertyChangeEvent>(_ => UpdateForm());
            return root;
        }

        // The explicit persist: marks the asset dirty and writes it to
        // disk. A drop already dirties the asset through
        // ApplyModifiedProperties, but the generation run reads the set
        // from disk, so the button forces the flush.
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

        // Undo of a drop is not a serialized property change event, so
        // the drop zone and the warning are rebuilt here; the bound
        // fields refresh themselves.
        private void OnUndoRedo()
        {
            serializedObject.Update();
            RebuildDropZone();
            UpdateForm();
        }

        private void UpdateForm()
        {
            if (m_outputEmptyWarning == null)
            {
                return;
            }
            // The set's fields are private, so reads go through the
            // serialized properties, not the target.
            serializedObject.Update();
            string output = serializedObject.FindProperty("m_OutputDirectory")?.stringValue;
            m_outputEmptyWarning.style.display =
                string.IsNullOrEmpty(output) ? DisplayStyle.Flex : DisplayStyle.None;

            // The conditional warnings: a randomization toggle is on but
            // the pool or reference it draws from is empty - the run
            // would silently skip that axis.
            Toggle(m_desksEmptyWarning,
                RandomizeOn("m_RandomizeDesks") && ArrayEmpty("m_Desks"));
            Toggle(m_skyboxesEmptyWarning,
                RandomizeOn("m_RandomizeSkyboxes") && ArrayEmpty("m_Skyboxes"));
        }

        private bool RandomizeOn(string propertyName)
        {
            return serializedObject.FindProperty(propertyName)?.boolValue == true;
        }

        private bool ArrayEmpty(string propertyName)
        {
            var array = serializedObject.FindProperty(propertyName);
            return array == null || array.arraySize == 0;
        }

        private void Toggle(Label warning, bool show)
        {
            warning.style.display = show ? DisplayStyle.Flex : DisplayStyle.None;
        }

        // The output-directory drop zone: the hint when empty, the live
        // path once set. A drop writes through the serialized property,
        // then rebuilds the zone and the warning.
        private void RebuildDropZone()
        {
            if (m_outputDirectorySlot == null)
            {
                return;
            }
            m_outputDirectorySlot.Clear();
            serializedObject.Update();
            string current = serializedObject.FindProperty("m_OutputDirectory")?.stringValue;

            var dropZone = new VisualElement();
            dropZone.AddToClassList("directory-dropzone");
            dropZone.Add(new Label(string.IsNullOrEmpty(current)
                ? "drop an Explorer/Finder directory here to set the output path"
                : current));
            dropZone.RegisterCallback<DragUpdatedEvent>(drag =>
            {
                DragAndDrop.visualMode = FirstDirectory() != null
                    ? DragAndDropVisualMode.Link
                    : DragAndDropVisualMode.Rejected;
                drag.StopPropagation();
            });
            dropZone.RegisterCallback<DragPerformEvent>(drag =>
            {
                var path = FirstDirectory();
                if (path == null)
                {
                    return;
                }
                serializedObject.Update();
                serializedObject.FindProperty("m_OutputDirectory").stringValue = path;
                serializedObject.ApplyModifiedProperties();
                DragAndDrop.AcceptDrag();
                drag.StopPropagation();
                RebuildDropZone();
                UpdateForm();
            });
            m_outputDirectorySlot.Add(dropZone);
        }

        // The first dropped path that is a directory on disk; null when
        // the drag holds only files. An Explorer/Finder drag populates
        // DragAndDrop.paths, never objectReferences.
        private static string FirstDirectory()
        {
            foreach (var path in DragAndDrop.paths)
            {
                if (Directory.Exists(path))
                {
                    return path;
                }
            }
            return null;
        }
    }
}
