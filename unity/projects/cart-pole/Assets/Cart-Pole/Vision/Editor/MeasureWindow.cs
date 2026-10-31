using UnityEditor;
using UnityEditor.UIElements;
using UnityEngine;
using UnityEngine.UIElements;

namespace CartPole.Vision.Editor
{
    // Measures the world-space offset between two scene objects: drop
    // them into From and To, press Measure, and the three fields show
    // To's world position minus From's, per world axis, in meters
    // (Unity's world frame, y up). The fields are read-only text, so a
    // value can be selected and copied (Ctrl+C) but not edited. Worked
    // example: From at (1, 2, 3) and To at (1.5, 2, 2) read 0.5, 0, -1.
    public sealed class MeasureWindow : EditorWindow
    {
        [MenuItem("Cart-Pole/Measure")]
        public static void Open()
        {
            GetWindow<MeasureWindow>("Measure");
        }

        private void CreateGUI()
        {
            var root = rootVisualElement;
            root.style.paddingLeft = 4f;
            root.style.paddingRight = 4f;
            root.style.paddingTop = 4f;

            var from = SceneObjectField("From");
            var to = SceneObjectField("To");
            root.Add(from);
            root.Add(to);

            var deltaX = ResultField("Delta X");
            var deltaY = ResultField("Delta Y");
            var deltaZ = ResultField("Delta Z");

            var measure = new Button(() => Measure(from, to, deltaX, deltaY, deltaZ)) { text = "Measure" };
            measure.style.marginTop = 4f;
            root.Add(measure);

            var heading = new Label("Result: To - From, world, meters");
            heading.style.marginTop = 8f;
            heading.style.unityFontStyleAndWeight = FontStyle.Bold;
            root.Add(heading);
            root.Add(deltaX);
            root.Add(deltaY);
            root.Add(deltaZ);
        }

        private static ObjectField SceneObjectField(string label)
        {
            return new ObjectField(label)
            {
                objectType = typeof(GameObject),
                allowSceneObjects = true,
            };
        }

        private static FloatField ResultField(string label)
        {
            return new FloatField(label) { isReadOnly = true };
        }

        private static void Measure(
            ObjectField from, ObjectField to, FloatField deltaX, FloatField deltaY, FloatField deltaZ)
        {
            if (from.value is not GameObject start || to.value is not GameObject end)
            {
                Debug.LogWarning("Measure: drop a scene object into both From and To");
                return;
            }
            var delta = end.transform.position - start.transform.position;
            deltaX.SetValueWithoutNotify(delta.x);
            deltaY.SetValueWithoutNotify(delta.y);
            deltaZ.SetValueWithoutNotify(delta.z);
        }
    }
}
