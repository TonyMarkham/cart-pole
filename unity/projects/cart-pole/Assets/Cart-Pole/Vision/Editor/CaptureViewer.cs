using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace CartPole.Vision.Editor
{
    // Review window for the capture pipeline's output: pages through the
    // frames in a capture directory, drawing each JPEG with its YOLO
    // label boxes overlaid - standard top-left-origin normalized
    // coordinates, the convention the capture component writes. Class
    // names come from the directory's classes.txt. No external tooling:
    // the label check stays inside Unity.
    public sealed class CaptureViewer : EditorWindow
    {
        private string directory = "Assets/Captures/cart-pole";

        private int index;

        private readonly List<string> frames = new List<string>();

        private readonly List<(int classId, float cx, float cy, float w, float h)> boxes =
            new List<(int, float, float, float, float)>();

        private Texture2D image;
        private string[] classNames = System.Array.Empty<string>();

        private static readonly Color[] Palette =
        {
            new Color(0.9f, 0.3f, 0.3f, 0.35f),
            new Color(0.3f, 0.9f, 0.3f, 0.35f),
            new Color(0.3f, 0.3f, 0.9f, 0.35f),
            new Color(0.9f, 0.9f, 0.2f, 0.35f),
            new Color(0.2f, 0.9f, 0.9f, 0.35f),
            new Color(0.9f, 0.2f, 0.9f, 0.35f),
        };

        [MenuItem("Cart-Pole/Capture Viewer")]
        public static void Open()
        {
            var window = GetWindow<CaptureViewer>("Capture Viewer");
            window.minSize = new Vector2(640f, 480f);
        }

        private void OnEnable()
        {
            if (frames.Count == 0 && Directory.Exists(directory))
            {
                Reload();
            }
        }

        private void OnGUI()
        {
            directory = EditorGUILayout.TextField("Directory", directory);

            using (new EditorGUILayout.HorizontalScope())
            {
                if (GUILayout.Button("Refresh", GUILayout.Width(80f)))
                {
                    Reload();
                }
                if (GUILayout.Button("<", GUILayout.Width(30f)))
                {
                    index = Mathf.Max(0, index - 1);
                    ShowFrame();
                }
                GUILayout.Label(
                    frames.Count == 0 ? "no frames" : $"{index + 1}/{frames.Count}",
                    GUILayout.Width(70f));
                if (GUILayout.Button(">", GUILayout.Width(30f)))
                {
                    index = Mathf.Min(frames.Count - 1, index + 1);
                    ShowFrame();
                }
                GUILayout.FlexibleSpace();
            }

            if (image == null)
            {
                EditorGUILayout.HelpBox(
                    "No frame loaded. Point the directory at a capture output and Refresh.",
                    MessageType.Info);
                return;
            }

            var area = GUILayoutUtility.GetRect(
                position.width - 16f, position.height - 90f);
            var scale = Mathf.Min(
                area.width / image.width, area.height / image.height);
            var draw = new Rect(
                area.center.x - image.width * scale / 2f,
                area.center.y - image.height * scale / 2f,
                image.width * scale,
                image.height * scale);
            GUI.DrawTexture(draw, image, ScaleMode.StretchToFill);

            foreach (var (classId, cx, cy, w, h) in boxes)
            {
                var box = new Rect(
                    draw.x + (cx - w / 2f) * draw.width,
                    draw.y + (cy - h / 2f) * draw.height,
                    w * draw.width,
                    h * draw.height);
                var color = Palette[classId % Palette.Length];
                EditorGUI.DrawRect(box, color);
                var label = Class(classId);
                var content = new GUIContent(label);
                var style = EditorStyles.miniLabel;
                var size = style.CalcSize(content);
                GUI.Label(
                    new Rect(box.x, box.y - size.y, size.x, size.y),
                    content, style);
            }
        }

        private void Reload()
        {
            frames.Clear();
            if (Directory.Exists(directory))
            {
                foreach (var file in Directory.GetFiles(directory, "frame_*.jpg"))
                {
                    frames.Add(file);
                }
                frames.Sort();
            }
            var classesPath = $"{directory}/classes.txt";
            classNames = File.Exists(classesPath)
                ? File.ReadAllLines(classesPath)
                : System.Array.Empty<string>();
            index = 0;
            ShowFrame();
        }

        private void ShowFrame()
        {
            boxes.Clear();
            if (image != null)
            {
                DestroyImmediate(image);
                image = null;
            }
            if (frames.Count == 0)
            {
                return;
            }
            index = Mathf.Clamp(index, 0, frames.Count - 1);
            var path = frames[index];
            if (File.Exists(path))
            {
                image = new Texture2D(2, 2);
                image.LoadImage(File.ReadAllBytes(path));
            }
            var labelPath = Path.ChangeExtension(path, ".txt");
            if (!File.Exists(labelPath))
            {
                return;
            }
            foreach (var line in File.ReadAllLines(labelPath))
            {
                var parts = line.Split(' ');
                if (parts.Length != 5 || !int.TryParse(parts[0], out var classId))
                {
                    continue;
                }
                boxes.Add((
                    classId,
                    float.Parse(parts[1]),
                    float.Parse(parts[2]),
                    float.Parse(parts[3]),
                    float.Parse(parts[4])));
            }
        }

        private string Class(int classId)
        {
            return classId >= 0 && classId < classNames.Length
                ? classNames[classId]
                : $"class {classId}";
        }
    }
}
