using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine.UIElements;

namespace CartPole.Vision.Editor
{
    // The desk asset's inspector: bound property fields plus the same
    // explicit Save affordance the marker editor has - dirty and flush,
    // so authored placement modifiers reach disk on demand. Pure
    // UIElements: the IMGUI-based default inspector embeds an
    // IMGUIContainer that trips the 6000.x global-event EndLayoutGroup
    // warning, so the fields are drawn with PropertyFields instead.
    [CustomEditor(typeof(DeskSO))]
    internal sealed class DeskSOEditor : UnityEditor.Editor
    {
        public override VisualElement CreateInspectorGUI()
        {
            var root = new VisualElement();
            foreach (var property in new[]
                     {
                         "label", "prefab", "position", "localEuler", "scale",
                     })
            {
                root.Add(new PropertyField(serializedObject.FindProperty(property)));
            }
            var save = new Button(Save) { text = "Save" };
            save.style.marginTop = 8f;
            root.Add(save);
            return root;
        }

        // The explicit persist: marks the asset dirty and writes it to
        // disk. Field edits already dirty the asset, but the import
        // pipeline only picks an SO up on an explicit save, so the
        // button forces the flush.
        private void Save()
        {
            EditorUtility.SetDirty(target);
            AssetDatabase.SaveAssetIfDirty(target);
        }
    }
}
