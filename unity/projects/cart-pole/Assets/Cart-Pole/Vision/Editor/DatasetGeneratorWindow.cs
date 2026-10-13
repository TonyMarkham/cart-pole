using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using UnityEditor;
using UnityEngine;

namespace CartPole.Vision.Editor
{
    // Synthetic dataset generator: samples random joint states within the
    // authored ranges, applies them through the JointDriver, renders the
    // capture camera and writes YOLO-format images + labels derived from
    // the placed ArUco marker quads.
    //
    // Class ids come from the marker name: Marker_<class>_<Label> in
    // ascending class order, e.g.
    //   Marker_0_LeftBase, Marker_1_RightBase,
    //   Marker_2_PolePivot, Marker_3_PoleEnd
    public sealed class DatasetGeneratorWindow : EditorWindow
    {
        private JointDriver driver;
        private Camera captureCamera;
        private string outputDirectory = "Assets/Dataset/cart-pole";
        private int frames = 500;
        private float unlimitedSpan = 3.0f;   // radians, for unlimited joints
        private int captureWidth = 960;
        private int captureHeight = 540;
        private int seed = 1234;

        private RenderTexture target;
        private Texture2D readback;

        [MenuItem("Cart-Pole/Dataset Generator")]
        public static void Open()
        {
            var window = GetWindow<DatasetGeneratorWindow>("Dataset Generator");
            window.minSize = new Vector2(480f, 320f);
        }

        private void OnGUI()
        {
            driver = (JointDriver)EditorGUILayout.ObjectField(
                "Joint driver", driver, typeof(JointDriver), true);
            captureCamera = (Camera)EditorGUILayout.ObjectField(
                "Capture camera", captureCamera, typeof(Camera), true);
            EditorGUILayout.HelpBox(
                "The camera must see the assembly. Marker quads named Marker_* " +
                "(from the ArUco Markers window) provide the labels.", MessageType.Info);

            frames = EditorGUILayout.IntField("Frames", frames);
            unlimitedSpan = EditorGUILayout.FloatField(
                "Unlimited joint span (rad)", unlimitedSpan);
            captureWidth = EditorGUILayout.IntField("Capture width", captureWidth);
            captureHeight = EditorGUILayout.IntField("Capture height", captureHeight);
            seed = EditorGUILayout.IntField("Seed", seed);
            outputDirectory = EditorGUILayout.TextField("Output", outputDirectory);

            if (GUILayout.Button("Generate dataset"))
            {
                Generate();
            }
        }

        private void Generate()
        {
            if (driver == null || captureCamera == null)
            {
                EditorUtility.DisplayDialog("Dataset", "Assign a JointDriver and a camera.", "OK");
                return;
            }
            driver.CaptureBasePose();

            var markers = FindMarkers();
            if (markers.Count == 0)
            {
                EditorUtility.DisplayDialog("Dataset",
                    "No Marker_* quads found under the assembly. Place markers first.", "OK");
                return;
            }

            Directory.CreateDirectory($"{outputDirectory}/images");
            Directory.CreateDirectory($"{outputDirectory}/labels");
            File.WriteAllText($"{outputDirectory}/classes.txt",
                "left-base\nright-base\npole-pivot\npole-end\n");

            target = new RenderTexture(captureWidth, captureHeight, 24);
            readback = new Texture2D(captureWidth, captureHeight, TextureFormat.RGB24, false);

            var previousActive = RenderTexture.active;
            var random = new System.Random(seed);
            var written = 0;

            try
            {
                for (var frame = 0; frame < frames; frame++)
                {
                    if (EditorUtility.DisplayCancelableProgressBar(
                            "Dataset generator", $"frame {frame}/{frames}",
                            frame / (float)frames))
                    {
                        Debug.LogWarning("[Dataset] cancelled by user");
                        break;
                    }

                    driver.Apply(SampleState(random));

                    RenderTexture.active = target;
                    captureCamera.targetTexture = target;
                    captureCamera.Render();
                    captureCamera.targetTexture = null;
                    readback.ReadPixels(new Rect(0, 0, captureWidth, captureHeight), 0, 0);
                    readback.Apply();
                    RenderTexture.active = previousActive;

                    var name = $"frame_{frame:D6}";
                    File.WriteAllBytes($"{outputDirectory}/images/{name}.png",
                        readback.EncodeToPNG());
                    File.WriteAllText($"{outputDirectory}/labels/{name}.txt",
                        BuildYoloLabel(markers, captureCamera));
                    written++;
                }
            }
            finally
            {
                RenderTexture.active = previousActive;
                DestroyImmediate(target);
                DestroyImmediate(readback);
                EditorUtility.ClearProgressBar();
            }

            AssetDatabase.Refresh();
            Debug.Log($"[Dataset] wrote {written} frames to {outputDirectory}");
        }

        private Dictionary<string, float> SampleState(System.Random random)
        {
            var state = new Dictionary<string, float>();
            foreach (var entry in driver.entries)
            {
                if (entry.joint == null || string.IsNullOrEmpty(entry.jointName))
                {
                    continue;
                }
                driver.GetRange(entry, unlimitedSpan, out var lo, out var hi);
                if (hi < lo)
                {
                    (lo, hi) = (hi, lo);
                }
                var span = hi - lo;
                state[entry.jointName] = (float)(lo + random.NextDouble() * span);
            }
            return state;
        }

        private List<Transform> FindMarkers()
        {
            var markers = new List<Transform>();
            if (driver.assemblyRoot == null)
            {
                return markers;
            }
            foreach (Transform node in driver.assemblyRoot.GetComponentsInChildren<Transform>(true))
            {
                if (node.name.StartsWith("Marker_"))
                {
                    markers.Add(node);
                }
            }
            markers.Sort((a, b) => string.CompareOrdinal(a.name, b.name));
            return markers;
        }

        private string BuildYoloLabel(List<Transform> markers, Camera camera)
        {
            var builder = new StringBuilder();
            foreach (var marker in markers)
            {
                if (!TryGetViewportBounds(marker, camera, out var bounds))
                {
                    continue;   // behind the camera or off-frame
                }
                builder.Append(ExtractClass(marker.name));
                builder.Append(' ');
                builder.Append(Clamp01(bounds.center.x).ToString("0.000000"));
                builder.Append(' ');
                // YOLO's origin is the image's top left; Unity's viewport
                // is bottom-up, so the center's y flips.
                builder.Append(Clamp01(1f - bounds.center.y).ToString("0.000000"));
                builder.Append(' ');
                builder.Append(Clamp01(bounds.size.x).ToString("0.000000"));
                builder.Append(' ');
                builder.Append(Clamp01(bounds.size.y).ToString("0.000000"));
                builder.Append('\n');
            }
            return builder.ToString();
        }

        // Projects the marker quad's four corners through the camera and
        // returns the normalized viewport AABB, or false when any corner is
        // behind the camera plane (marker not fully visible).
        private static bool TryGetViewportBounds(Transform marker, Camera camera, out Bounds bounds)
        {
            bounds = new Bounds();
            var corners = new[]
            {
                new Vector3(-0.5f, -0.5f, 0f),
                new Vector3(0.5f, -0.5f, 0f),
                new Vector3(0.5f, 0.5f, 0f),
                new Vector3(-0.5f, 0.5f, 0f),
            };
            var min = new Vector2(float.MaxValue, float.MaxValue);
            var max = new Vector2(float.MinValue, float.MinValue);
            foreach (var corner in corners)
            {
                var world = marker.TransformPoint(corner);
                var viewport = camera.WorldToViewportPoint(world);
                if (viewport.z <= 0f)
                {
                    return false;
                }
                var point = new Vector2(viewport.x, viewport.y);
                min = Vector2.Min(min, point);
                max = Vector2.Max(max, point);
            }
            bounds.SetMinMax(min, max);
            return true;
        }

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
