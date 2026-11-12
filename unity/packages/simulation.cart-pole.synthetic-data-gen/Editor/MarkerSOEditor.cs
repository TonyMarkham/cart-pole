using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace Simulation.CartPole.SyntheticDataGen.Editor
{
    // The marker asset's inspector: the bound UXML form. The anchor is a
    // SiblingPath, drawn by the shared SiblingPathDrawer
    // (Greyhound.Utilities): dropping a part sets the path's root (the
    // STEP assembly) and its indices. This editor adds the size rule
    // (size > 0) and a preview of the marker texture. The base type is
    // qualified: the enclosing namespace's trailing ".Editor" segment
    // would otherwise bind the bare name to itself.
    [CustomEditor(typeof(MarkerSO))]
    internal sealed class MarkerSOEditor : UnityEditor.Editor
    {
        [SerializeField] private VisualTreeAsset m_VisualTreeAsset;

        [SerializeField] private StyleSheet m_StyleSheet;

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

            m_sizeWarning = root.Q<Label>("size-warning");

            m_prefabPreview = new Image { scaleMode = ScaleMode.ScaleToFit };
            m_prefabPreview.AddToClassList("texture-preview");
            root.Q<VisualElement>("prefab-slot")?.Add(m_prefabPreview);

            UpdateForm();
            root.Q<Button>("save-button")?.RegisterCallback<ClickEvent>(_ => Save());
            root.RegisterCallback<SerializedPropertyChangeEvent>(_ => UpdateForm());
            return root;
        }

        // The explicit persist: marks the asset dirty and writes it to
        // disk. Edits already dirty the asset through
        // ApplyModifiedProperties, but the import pipeline only picks an
        // SO up on an explicit save, so the button forces the flush.
        private void Save()
        {
            EditorUtility.SetDirty(target);
            AssetDatabase.SaveAssetIfDirty(target);
        }

        private void OnEnable()
        {
            Undo.undoRedoPerformed += UpdateForm;
        }

        private void OnDisable()
        {
            Undo.undoRedoPerformed -= UpdateForm;
        }

        private void UpdateForm()
        {
            var marker = (MarkerSO)target;
            if (marker == null)
            {
                return;
            }
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
    }
}
