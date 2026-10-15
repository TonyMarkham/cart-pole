using System;
using System.Collections.Generic;
using System.IO;

using UnityEditor;
using UnityEngine;
using UnityEngine.UIElements;

namespace CartPole.Vision.Editor
{
    // The end-of-run report, viewable in the editor: reads a dataset
    // directory (the per-state frame_NNNNNN.json files plus the
    // verification stats) and presents every capture - state, scene
    // elements, and how many labels survived the OpenCV check - with
    // frames that dropped labels flagged red. Also lists the zero-marker
    // crops when the run produced any.
    public sealed class CaptureReportWindow : EditorWindow
    {
        [SerializeField] private string directory = "Assets/Captures/cart-pole";

        private readonly List<RowData> rows = new List<RowData>();
        private readonly List<string> zeroCrops = new List<string>();
        private int totalDropped;

        private static readonly string[] Headers =
            { "Frame", "Situation", "Cart", "Pole", "Desk", "Skybox", "Dropped" };
        private static readonly float[] Widths = { 110f, 150f, 55f, 55f, 110f, 150f, 70f };

        private sealed class RowData
        {
            public string frame;
            public string situation;
            public string cart;
            public string pole;
            public string desk;
            public string skybox;
            public int dropped;
            public string droppedMarkers;
        }

        [MenuItem("Cart-Pole/Run Report")]
        public static void Open()
        {
            var window = GetWindow<CaptureReportWindow>("Run Report");
            window.minSize = new Vector2(820f, 420f);
        }

        private void OnEnable()
        {
            if (rows.Count == 0 && Directory.Exists(directory))
            {
                Load();
            }
        }

        private void CreateGUI()
        {
            var root = rootVisualElement;
            root.Clear();

            var bar = new VisualElement();
            bar.style.flexDirection = FlexDirection.Row;
            var dirField = new TextField("Directory") { value = directory };
            dirField.style.flexGrow = 1f;
            bar.Add(dirField);
            var refresh = new Button(() =>
            {
                directory = dirField.value;
                Load();
                CreateGUI();
            }) { text = "Refresh" };
            bar.Add(refresh);
            root.Add(bar);

            var summary = new Label(
                $"{rows.Count} frames | {totalDropped} dropped");
            summary.style.marginTop = 4f;
            summary.style.unityFontStyleAndWeight = FontStyle.Bold;
            root.Add(summary);

            var header = HeaderRow();
            header.style.backgroundColor = new Color(0.25f, 0.25f, 0.25f);
            root.Add(header);

            var list = new ListView(rows, 22f, MakeRow, BindRow);
            list.style.flexGrow = 1f;
            root.Add(list);

            if (zeroCrops.Count > 0)
            {
                var zeroHeader = new Label(
                    $"Zero-marker crops ({zeroCrops.Count}):");
                zeroHeader.style.marginTop = 6f;
                zeroHeader.style.unityFontStyleAndWeight = FontStyle.Bold;
                root.Add(zeroHeader);
                var zeroLabel = new Label(string.Join(",  ", zeroCrops));
                zeroLabel.style.whiteSpace = WhiteSpace.Normal;
                zeroLabel.style.color = new Color(1f, 0.55f, 0.3f);
                root.Add(zeroLabel);
            }
        }

        private static VisualElement HeaderRow()
        {
            var row = new VisualElement();
            row.style.flexDirection = FlexDirection.Row;
            for (var i = 0; i < Headers.Length; i++)
            {
                var label = new Label(Headers[i]);
                label.style.width = Widths[i];
                label.style.unityFontStyleAndWeight = FontStyle.Bold;
                row.Add(label);
            }
            return row;
        }

        private static VisualElement MakeRow()
        {
            var row = new VisualElement();
            row.style.flexDirection = FlexDirection.Row;
            foreach (var w in Widths)
            {
                var label = new Label();
                label.style.width = w;
                label.style.overflow = Overflow.Hidden;
                row.Add(label);
            }
            return row;
        }

        private void BindRow(VisualElement row, int index)
        {
            var data = rows[index];
            var droppedText = data.dropped < 0
                ? "?"
                : data.droppedMarkers ?? data.dropped.ToString();
            var values = new[]
            {
                data.frame, data.situation, data.cart, data.pole,
                data.desk, data.skybox, droppedText,
            };
            for (var i = 0; i < values.Length; i++)
            {
                var label = (Label)row[i];
                label.text = values[i];
                label.style.color = i == values.Length - 1 && data.dropped != 0
                    ? new Color(1f, 0.4f, 0.35f)
                    : Color.white;
            }
        }

        // The marker crops live in the wip~ sidecar directory (the
        // folder Unity's AssetDatabase ignores); corpora captured
        // before that layout keep them at the dataset root.
        private static string CropPath(string directory, string stem, int id)
        {
            var wip = Path.Combine(
                directory, "wip~", $"{stem}_8k_marker_{id:0000}.jpg");
            return File.Exists(wip)
                ? wip
                : Path.Combine(directory, $"{stem}_8k_marker_{id:0000}.jpg");
        }

        private void Load()
        {
            rows.Clear();
            zeroCrops.Clear();
            totalDropped = 0;
            if (!Directory.Exists(directory))
            {
                return;
            }

            // The authored marker classes, from markers.json - the
            // missing 8K crops per frame name the dropped markers.
            // Sidecars live in the wip~ directory Unity ignores;
            // legacy corpora keep them at the dataset root.
            var markerDefs = new List<MarkerDef>();
            var markersPath = Path.Combine(directory, "wip~", "markers.json");
            if (!File.Exists(markersPath))
            {
                markersPath = Path.Combine(directory, "markers.json");
            }
            if (File.Exists(markersPath))
            {
                var list = JsonUtility.FromJson<MarkerList>(
                    File.ReadAllText(markersPath));
                if (list?.markers != null)
                {
                    foreach (var m in list.markers)
                    {
                        markerDefs.Add(new MarkerDef
                        {
                            classId = m.classId,
                            label = m.label,
                            id = m.id,
                        });
                    }
                }
            }

            // The per-frame state jsons live in the wip~ sidecar
            // directory; legacy corpora keep them at the dataset root.
            var wip = Path.Combine(directory, "wip~");
            var jsonFiles = Directory.Exists(wip)
                ? Directory.GetFiles(wip, "frame_*_state.json")
                : Array.Empty<string>();
            if (jsonFiles.Length == 0)
            {
                jsonFiles = Directory.GetFiles(directory, "frame_*_state.json");
            }
            foreach (var jsonPath in jsonFiles)
            {
                var state = JsonUtility.FromJson<StateEntry>(
                    File.ReadAllText(jsonPath));
                if (state == null)
                {
                    continue;
                }
                var stem = Path.GetFileNameWithoutExtension(jsonPath)
                    .Replace("_state", "");
                var missing = new List<string>();
                foreach (var def in markerDefs)
                {
                    var cropPath = CropPath(directory, stem, def.id);
                    if (!File.Exists(cropPath))
                    {
                        missing.Add($"{def.id:0000}");
                    }
                }
                rows.Add(new RowData
                {
                    frame = stem,
                    situation = state.situation,
                    cart = state.cartPosition.ToString("0.000"),
                    pole = state.poleRotation.ToString("0.000"),
                    desk = state.useDesk ? Path.GetFileNameWithoutExtension(state.deskPath) : "-",
                    skybox = state.useSkybox
                        ? Path.GetFileNameWithoutExtension(state.skyboxPath)
                        : "-",
                    dropped = missing.Count,
                    droppedMarkers = string.Join(", ", missing),
                });
                totalDropped += missing.Count;
            }
            rows.Sort((a, b) => string.CompareOrdinal(a.frame, b.frame));

            var zeroPath = Path.Combine(directory, "wip~", "zero_marker_crops.txt");
            if (!File.Exists(zeroPath))
            {
                zeroPath = Path.Combine(directory, "zero_marker_crops.txt");
            }
            if (File.Exists(zeroPath))
            {
                foreach (var line in File.ReadAllLines(zeroPath))
                {
                    if (!string.IsNullOrEmpty(line))
                    {
                        zeroCrops.Add(line);
                    }
                }
            }
        }

        [Serializable]
        private sealed class MarkerDef
        {
            public int classId;
            public string label;
            public int id;
        }

        [Serializable]
        private sealed class MarkerList
        {
            public List<MarkerDef> markers;
        }

        [Serializable]
        private sealed class StateEntry
        {
            public string frame;
            public string situation;
            public float cartPosition;
            public float poleRotation;
            public bool useDesk;
            public string deskPath;
            public bool useSkybox;
            public string skyboxPath;
            public float skyboxYaw;
            public bool useLight;
            public float lightIntensity;
            public float lightYaw;
            public bool useCameraPosition;
            public Vector3 cameraPosition;
            public Vector3 cameraRotation;
        }

        [Serializable]
        private sealed class VerificationStatsFile
        {
            public string dataset;
            public int framesScanned;
            public int labelsConfirmed;
            public int labelsDropped;
            public VerificationFrameFile[] frames;
        }

        [Serializable]
        private sealed class VerificationFrameFile
        {
            public string frame;
            public int confirmed;
            public int dropped;
            public string droppedMarkers;
        }
    }
}
