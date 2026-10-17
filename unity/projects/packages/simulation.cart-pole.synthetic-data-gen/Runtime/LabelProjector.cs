using System.Collections.Generic;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // The label source: the applied marker quads projected through a
    // camera into YOLO lines. The caller must project while the camera
    // is in the configuration that rendered the image - the capture
    // camera is physical (Pi GS sensor gate 4.968x3.726 mm, 6 mm
    // lens), and a gate fit makes the vertical projection target-
    // aspect-dependent: projecting after the target restore re-
    // projects the boxes in the Game view's aspect (measured skew:
    // boxes ~1.5x too tall). The lines are proposals - a line lands
    // in the dataset only when the verification gate confirms it.
    internal static class LabelProjector
    {
        // The marker prefab's quad corners in the quad's local space.
        private static readonly Vector3[] QuadCorners =
        {
            new(-0.5f, -0.5f, 0f),
            new(0.5f, -0.5f, 0f),
            new(0.5f, 0.5f, 0f),
            new(-0.5f, 0.5f, 0f),
        };

        // One line per fully visible marker: "class x y w h", all four
        // coordinates normalized to [0, 1], YOLO's top-left origin (the
        // center's y flips from Unity's bottom-up viewport). A marker
        // with any corner behind the camera plane is skipped.
        internal static List<string> Project(
            IReadOnlyList<Transform> markers,
            Camera camera,
            IReadOnlyDictionary<int, int> classTags)
        {
            var lines = new List<string>();
            foreach (var marker in markers)
            {
                if (!TryViewportBounds(marker, camera, out var bounds))
                {
                    continue;
                }
                var classId = ExtractClass(marker.name);
                if (!classTags.ContainsKey(classId))
                {
                    continue;
                }
                lines.Add(
                    $"{classId} {Clamp01(bounds.center.x):0.000000} " +
                    $"{Clamp01(1f - bounds.center.y):0.000000} " +
                    $"{Clamp01(bounds.size.x):0.000000} " +
                    $"{Clamp01(bounds.size.y):0.000000}");
            }
            return lines;
        }

        // The projected viewport AABB of a marker's quad, or false when
        // any corner is behind the camera plane (marker not fully
        // visible).
        private static bool TryViewportBounds(
            Transform marker, Camera camera, out Bounds bounds)
        {
            bounds = new Bounds();
            var min = new Vector2(float.MaxValue, float.MaxValue);
            var max = new Vector2(float.MinValue, float.MinValue);
            foreach (var corner in QuadCorners)
            {
                var viewport = camera.WorldToViewportPoint(
                    marker.TransformPoint(corner));
                if (viewport.z <= 0f)
                {
                    return false;
                }
                min = Vector2.Min(min, new Vector2(viewport.x, viewport.y));
                max = Vector2.Max(max, new Vector2(viewport.x, viewport.y));
            }
            bounds.SetMinMax(min, max);
            return true;
        }

        // Marker_<class>_<Label>, applied in ascending class order.
        private static int ExtractClass(string markerName)
        {
            var start = "Marker_".Length;
            var end = markerName.IndexOf('_', start);
            if (end < 0)
            {
                end = markerName.Length;
            }
            return int.Parse(markerName[start..end]);
        }

        private static float Clamp01(float value)
        {
            return Mathf.Clamp(value, 0f, 1f);
        }
    }
}
