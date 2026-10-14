using System;
using System.Collections.Generic;
using System.IO;
using System.Text;
using UnityEngine;

namespace CartPole.Vision
{
    // Screenshot sidecar for the rig-state pipeline: subscribes to the
    // randomizer's StateApplied event and, on each application, renders
    // the assigned camera, writes a JPEG and a YOLO label derived from
    // the placed ArUco marker quads. Where the images land is this
    // component's outputDirectory - the randomizer itself writes
    // nothing. Play mode only; the subscription lives with the
    // component's enable state.
    //
    // Label math: each marker quad's four corners are projected through
    // the capture camera (WorldToViewportPoint, a pinhole model - FOV
    // and aspect, no lens distortion) into a normalized viewport AABB.
    // A marker with any corner behind the camera plane is skipped; the
    // rest clamp to the frame.
    public sealed class RigStateCapture : MonoBehaviour
    {
        [SerializeField] private RigStateRandomizer randomizer;

        [SerializeField] private Camera captureCamera;

        // The MarkerManager whose rig the marker quads hang off; the
        // label source is resolved beneath its scene root. Falls back to
        // the same GameObject when unset.
        [SerializeField] private MarkerManager markerManager;

        // Camera position jitter: the capture position wanders up to
        // this many meters per axis around the authored camera position,
        // re-aimed at the RobotBounds cube's center - with a frustum
        // check that shrinks the offset until the cube stays in frame.
        [SerializeField] private float positionJitter = 0.15f;

        [SerializeField] private int jitterSeed;

        // Floor guard: the jittered camera never goes below this height,
        // so a random offset cannot push it under the ground plane.
        [SerializeField] private float minCameraHeight = 0.05f;

        // The workspace envelope: a scene-placed RobotBounds cube
        // (position = center, scale = size) defined once so the frustum
        // contract can be fitted visually. Fixed - not per-pose - so the
        // framing stays consistent across frames.
        [SerializeField] private RobotBounds robotBounds;

        [SerializeField] private string outputDirectory = "Assets/Captures/cart-pole";

        [SerializeField] private int captureWidth = 960;

        [SerializeField] private int captureHeight = 540;

        // JPEG only, by decision: the physical camera route feeds the
        // detector lossy JPEGs, so the synthetic set must share the same
        // lossy statistics - no PNG mixing. Quality matches the live
        // StateServer loop (EncodeToJPG(90)).
        [SerializeField, Range(1, 100)] private int jpegQuality = 90;

        private RenderTexture target;
        private Texture2D readback;
        private List<Transform> markers;
        private Transform markersRoot;
        private Vector3 cameraBasePosition;
        private System.Random jitterRandom;
        private RigStateRandomizer.StateEvent lastEvent;
        private string lastFrame;
        private readonly Dictionary<string, string> arucoDroppedMarkers =
            new Dictionary<string, string>();
        private bool warnedNoBounds;
        private bool markersJsonWritten;
        private Dictionary<int, int> classTagIds;
        private Camera markerCamera;
        private RenderTexture markerTarget;
        private Texture2D markerReadback;
        private int frames;

        private void OnEnable()
        {
            if (randomizer == null || captureCamera == null)
            {
                Debug.LogWarning("[RigStateCapture] assign the randomizer and the capture camera");
                return;
            }
            cameraBasePosition = captureCamera.transform.position;
            jitterRandom = jitterSeed == 0
                ? new System.Random()
                : new System.Random(jitterSeed);
            Directory.CreateDirectory(outputDirectory);
            target = new RenderTexture(captureWidth, captureHeight, 24);
            readback = new Texture2D(captureWidth, captureHeight, TextureFormat.RGB24, false);
            randomizer.StateApplied += OnStateApplied;
            randomizer.RunCompleted += OnRunCompleted;
            Debug.Log($"[RigStateCapture] saving to {outputDirectory}");
        }

        private void OnDisable()
        {
            if (randomizer != null)
            {
                randomizer.StateApplied -= OnStateApplied;
                randomizer.RunCompleted -= OnRunCompleted;
            }
            if (frames > 0)
            {
                Debug.Log($"[RigStateCapture] wrote {frames} frames to {outputDirectory}");
            }
            if (target != null)
            {
                Destroy(target);
                target = null;
            }
            if (readback != null)
            {
                Destroy(readback);
                readback = null;
            }
        }

        private void OnStateApplied(RigStateRandomizer.StateEvent e)
        {
            lastEvent = e;
            WriteMarkersJson();
            Capture();
        }

        private int arucoFrames;
        private int arucoKept;
        private int arucoDropped;
        private readonly List<VerificationFrame> arucoPerFrame =
            new List<VerificationFrame>();

        [Serializable]
        private sealed class VerificationFrame
        {
            public string frame;
            public int confirmed;
            public int dropped;
            public string droppedMarkers;
        }

        [Serializable]
        private sealed class VerificationStats
        {
            public string dataset;
            public int framesScanned;
            public int labelsConfirmed;
            public int labelsDropped;
            public VerificationFrame[] frames;
        }

        private static int ParseCount(string data, string keyword)
        {
            var parts = data.Split(' ');
            for (var i = 1; i < parts.Length; i++)
            {
                if (parts[i].StartsWith(keyword) &&
                    int.TryParse(parts[i - 1], out var value))
                {
                    return value;
                }
            }
            return 0;
        }

        // Run over: the OpenCV verification runs automatically - the
        // checker saves a cropped tag scan per detected 4K marker and
        // rewrites the labels, dropping every box whose tag cannot be
        // re-verified from its own crop (cropped scans are the only
        // verification basis).
        private void OnRunCompleted()
        {
#if UNITY_EDITOR
            var projectRoot = Directory.GetParent(Application.dataPath).FullName;
            var checker = Path.Combine(projectRoot, "Tools", "aruco_check.py");
            var uvCandidates = new[] { "uv", "/home/tony/.local/bin/uv" };
            foreach (var uv in uvCandidates)
            {
                var psi = new System.Diagnostics.ProcessStartInfo
                {
                    FileName = uv,
                    Arguments = $"run --with opencv-python python \"{checker}\" \"{outputDirectory}\" --write",
                    WorkingDirectory = projectRoot,
                    RedirectStandardOutput = true,
                    RedirectStandardError = true,
                    UseShellExecute = false,
                    CreateNoWindow = true,
                };
                System.Diagnostics.Process process;
                try
                {
                    process = System.Diagnostics.Process.Start(psi);
                }
                catch (System.ComponentModel.Win32Exception)
                {
                    continue;   // uv not on this PATH - try the next candidate
                }
                process.OutputDataReceived += (_, line) =>
                {
                    var data = line?.Data;
                    if (string.IsNullOrEmpty(data))
                    {
                        return;
                    }
                    Debug.Log($"[aruco_check] {data}");
                    if (data.StartsWith("frame_") &&
                        data.Contains("confirmed") && data.Contains("dropped"))
                    {
                        // "frame_000000.jpg: 2 confirmed, 1 dropped"
                        arucoFrames++;
                        arucoKept += ParseCount(data, "confirmed");
                        arucoDropped += ParseCount(data, "dropped");
                        lastFrame = data.Substring(0, data.IndexOf(".jpg"));
                        arucoPerFrame.Add(new VerificationFrame
                        {
                            frame = lastFrame,
                            confirmed = ParseCount(data, "confirmed"),
                            dropped = ParseCount(data, "dropped"),
                        });
                    }
                    else if (data.StartsWith("dropped_markers: "))
                    {
                        // "dropped_markers: Pivot, End" - names the
                        // unrecognized markers of the preceding frame.
                        if (arucoPerFrame.Count > 0)
                        {
                            arucoPerFrame[^1].droppedMarkers =
                                data["dropped_markers: ".Length..];
                        }
                    }
                    else if (data.StartsWith("done:"))
                    {
                        // "done: 23 kept, 6 dropped (...)"
                        arucoKept = ParseCount(data, "kept");
                        arucoDropped = ParseCount(data, "dropped");
                        File.WriteAllText(
                            $"{outputDirectory}/verification_stats.json",
                            JsonUtility.ToJson(new VerificationStats
                            {
                                dataset = outputDirectory,
                                framesScanned = arucoFrames,
                                labelsConfirmed = arucoKept,
                                labelsDropped = arucoDropped,
                                frames = arucoPerFrame.ToArray(),
                            }, true) + "\n");
                        Debug.Log(
                            "[RigStateCapture] verification stats: " +
                            $"{arucoFrames} frames scanned, {arucoKept} labels " +
                            $"confirmed, {arucoDropped} labels dropped " +
                            "- saved to verification_stats.json");
                    }
                };
                process.ErrorDataReceived += (_, line) =>
                {
                    if (!string.IsNullOrEmpty(line?.Data))
                    {
                        Debug.LogWarning($"[aruco_check] {line.Data}");
                    }
                };
                process.BeginOutputReadLine();
                process.BeginErrorReadLine();
                return;
            }
            Debug.LogError("[RigStateCapture] could not start uv - OpenCV verification not run");
#endif
        }

        // The class-to-tag mapping, sourced from the rig's authored
        // MarkerSOs (class = rig slot order, tag id = the SO's arUcoId)
        // - the OpenCV checker verifies labels against the SOs, not
        // against asset filenames.
        [Serializable]
        private sealed class MarkerEntry
        {
            public int classId;
            public string label;
            public int id;
        }

        [Serializable]
        private sealed class MarkerList
        {
            public List<MarkerEntry> markers;
        }

        private void WriteMarkersJson()
        {
            if (markersJsonWritten || markerManager == null ||
                markerManager.rig == null)
            {
                return;
            }
            markersJsonWritten = true;
            var list = new MarkerList { markers = new List<MarkerEntry>() };
            var classIndex = 0;
            foreach (var marker in markerManager.rig.Markers)
            {
                if (marker == null || marker.TagId < 0)
                {
                    continue;
                }
                // Python-friendly form: the plain int id OpenCV reports,
                // exported here - not reassembled in Python.
                list.markers.Add(new MarkerEntry
                {
                    classId = classIndex,
                    label = marker.label,
                    id = marker.TagId,
                });
                classIndex++;
            }
            File.WriteAllText(
                $"{outputDirectory}/markers.json",
                JsonUtility.ToJson(list, true));
        }

        private void Capture()
        {
            if (target == null)
            {
                return;
            }
            EnsureMarkers();
            if (robotBounds == null)
            {
                robotBounds = UnityEngine.Object.FindAnyObjectByType<RobotBounds>();
            }
            // A forced situation with an authored camera position places
            // the capture camera exactly there and skips the jitter for
            // that frame - the situation reproduces the shot verbatim.
            if (lastEvent != null && lastEvent.hasCameraPosition)
            {
                captureCamera.transform.SetPositionAndRotation(
                    lastEvent.cameraPosition,
                    Quaternion.Euler(lastEvent.cameraRotation));
            }
            // The bounds cube is a real mesh: hidden for the duration of
            // the render, restored after, so it never pollutes a frame
            // even while its renderer is on for scene editing.
            MeshRenderer boundsRenderer = null;
            var boundsWasVisible = false;
            if (robotBounds != null)
            {
                boundsRenderer = robotBounds.GetComponent<MeshRenderer>();
                boundsWasVisible = boundsRenderer != null && boundsRenderer.enabled;
                if (boundsWasVisible)
                {
                    boundsRenderer.enabled = false;
                }
            }
            if (lastEvent == null || !lastEvent.hasCameraPosition)
            {
                JitterCamera();
            }
            var previousActive = RenderTexture.active;
            var previousTarget = captureCamera.targetTexture;
            captureCamera.targetTexture = target;
            captureCamera.Render();
            captureCamera.targetTexture = previousTarget;
            RenderTexture.active = target;
            readback.ReadPixels(new Rect(0, 0, captureWidth, captureHeight), 0, 0);
            RenderTexture.active = previousActive;
            readback.Apply();
            var name = $"frame_{frames:D6}";
            File.WriteAllBytes($"{outputDirectory}/{name}.jpg",
                readback.EncodeToJPG(jpegQuality));
            WriteLabel(name);
            WriteManifest(name);
            CaptureMarker8K(name);
            frames++;
            if (boundsWasVisible && boundsRenderer != null)
            {
                boundsRenderer.enabled = true;
            }
        }

        // One JSON per frame, named like the frame itself
        // (frame_NNNNNN.json): the state that was applied, what the
        // randomizer spawned around it, and where the camera stood -
        // human-navigable, one state per file.
        [Serializable]
        private sealed class ManifestEntry
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

        // Marker close-ups: a telephoto camera sharing the capture
        // camera's transform, FOV auto-fitted each state to frame all
        // four markers tightly - the narrow view turns the capture
        // resolution into high-definition per-marker images. One crop
        // per marker is written next to the frame.
        private static readonly Vector3[] QuadCorners =
        {
            new(-0.5f, -0.5f, 0f),
            new(0.5f, -0.5f, 0f),
            new(0.5f, 0.5f, 0f),
            new(-0.5f, 0.5f, 0f),
        };

        // An 8K (7680x4320, 16:9) render of the scene through a camera
        // sharing the capture camera's transform and FOV - the same view
        // at nearly 3x the pixel density, so the markers are large
        // enough for OpenCV to scan reliably.
        private const int Marker8KWidth = 7680;
        private const int Marker8KHeight = 4320;

        private void CaptureMarker8K(string name)
        {
            if (markers == null || markers.Count == 0)
            {
                return;
            }
            if (markerCamera == null)
            {
                CreateMarkerCamera();
            }
            markerCamera.transform.SetParent(captureCamera.transform, false);
            markerCamera.transform.localPosition = Vector3.zero;
            markerCamera.transform.localRotation = Quaternion.identity;
            markerCamera.fieldOfView = captureCamera.fieldOfView;

            markerCamera.Render();
            var previousActive = RenderTexture.active;
            RenderTexture.active = markerTarget;
            markerReadback.ReadPixels(
                new Rect(0, 0, Marker8KWidth, Marker8KHeight), 0, 0);
            markerReadback.Apply();
            RenderTexture.active = previousActive;
            File.WriteAllBytes(
                $"{outputDirectory}/{name}_8k.jpg",
                markerReadback.EncodeToJPG(jpegQuality));

            // Retain a crop of every marker: the pixel rectangle comes
            // from the exact projected quad corners - no detection
            // involved, so every marker in every frame gets its crop
            // file.
            var toLocal = markerCamera.transform.worldToLocalMatrix;
            foreach (var marker in markers)
            {
                var min = new Vector2(float.MaxValue, float.MaxValue);
                var max = new Vector2(float.MinValue, float.MinValue);
                var valid = true;
                foreach (var corner in QuadCorners)
                {
                    var viewport = markerCamera.WorldToViewportPoint(
                        marker.TransformPoint(corner));
                    if (viewport.z <= 0f)
                    {
                        valid = false;
                        break;
                    }
                    min = Vector2.Min(min, new Vector2(viewport.x, viewport.y));
                    max = Vector2.Max(max, new Vector2(viewport.x, viewport.y));
                }
                if (!valid)
                {
                    continue;
                }
                var classId = ExtractClass(marker.name);
                if (!classTagIds.TryGetValue(classId, out var tagId))
                {
                    continue;
                }
                var x0 = Mathf.Clamp(
                    Mathf.FloorToInt(min.x * Marker8KWidth) - 20, 0, Marker8KWidth - 1);
                var yTop = Mathf.Clamp(
                    Mathf.FloorToInt((1f - max.y) * Marker8KHeight) - 20, 0, Marker8KHeight - 1);
                var cw = Mathf.Clamp(
                    Mathf.CeilToInt((max.x - min.x) * Marker8KWidth) + 40, 1, Marker8KWidth - x0);
                var ch = Mathf.Clamp(
                    Mathf.CeilToInt((max.y - min.y) * Marker8KHeight) + 40, 1, Marker8KHeight - yTop);
                var yGl = Marker8KHeight - yTop - ch;
                var block = markerReadback.GetPixels(x0, yGl, cw, ch);
                var crop = new Texture2D(cw, ch, TextureFormat.RGB24, false);
                crop.SetPixels(block);
                crop.Apply();
                File.WriteAllBytes(
                    $"{outputDirectory}/{name}_8k_marker_{tagId:0000}.jpg",
                    crop.EncodeToJPG(jpegQuality));
                DestroyImmediate(crop);
            }
        }

        private void CreateMarkerCamera()
        {
            var go = new GameObject("MarkerCamera");
            go.hideFlags = HideFlags.HideAndDontSave;
            go.transform.SetParent(captureCamera.transform, false);
            var cam = go.AddComponent<Camera>();
            cam.CopyFrom(captureCamera);
            cam.enabled = false;
            markerTarget = new RenderTexture(Marker8KWidth, Marker8KHeight, 24);
            cam.targetTexture = markerTarget;
            markerReadback = new Texture2D(
                Marker8KWidth, Marker8KHeight, TextureFormat.RGB24, false);
            markerCamera = cam;
        }

        private void WriteManifest(string name)
        {
            var entry = new ManifestEntry
            {
                frame = name,
                situation = lastEvent?.situation ?? "",
                cartPosition = lastEvent?.cartPosition ?? 0f,
                poleRotation = lastEvent?.poleRotation ?? 0f,
                useDesk = lastEvent?.useDesk ?? false,
                deskPath = AssetPathOf(lastEvent?.deskAsset),
                useSkybox = lastEvent?.useSkybox ?? false,
                skyboxPath = AssetPathOf(lastEvent?.skyboxAsset),
                skyboxYaw = lastEvent?.skyboxYaw ?? 0f,
                useLight = lastEvent?.useLight ?? false,
                lightIntensity = lastEvent?.lightIntensity ?? 0f,
                lightYaw = lastEvent?.lightYaw ?? 0f,
                useCameraPosition = lastEvent != null && lastEvent.hasCameraPosition,
                cameraPosition = captureCamera.transform.position,
                cameraRotation = captureCamera.transform.eulerAngles,
            };
            File.WriteAllText(
                $"{outputDirectory}/{name}_state.json",
                JsonUtility.ToJson(entry, true) + "\n");
        }

        // The project path of an applied asset, so a ForcedSituationSO
        // can be authored with the exact references this run used.
        private static string AssetPathOf(UnityEngine.Object asset)
        {
#if UNITY_EDITOR
            return asset != null ? UnityEditor.AssetDatabase.GetAssetPath(asset) : "";
#else
            return "";
#endif
        }

        // The ArUco quads are the label source; resolved lazily on the
        // first capture. An empty resolution is not cached - the state
        // pipeline can fire before the MarkerManager's Start has applied
        // the markers, so the resolve retries until the quads exist.
        private void EnsureMarkers()
        {
            if (markers is { Count: > 0 })
            {
                return;
            }
            markers = new List<Transform>();
            if (markerManager == null)
            {
                markerManager = GetComponent<MarkerManager>();
            }
            var root = markerManager != null ? markerManager.ResolveSceneRoot() : null;
            markersRoot = root;
            classTagIds = new Dictionary<int, int>();
            if (markerManager.rig != null)
            {
                var classIndex = 0;
                foreach (var marker in markerManager.rig.Markers)
                {
                    if (marker == null)
                    {
                        continue;
                    }
                    classTagIds[classIndex] = marker.TagId;
                    classIndex++;
                }
            }
            if (root == null)
            {
                Debug.LogWarning("[RigStateCapture] no assembly root - labels will be empty");
                markers = null;
                return;
            }
            foreach (Transform node in root.GetComponentsInChildren<Transform>(true))
            {
                if (node.name.StartsWith("Marker_"))
                {
                    markers.Add(node);
                }
            }
            if (markers.Count == 0)
            {
                Debug.LogWarning("[RigStateCapture] no Marker_* quads under the assembly yet - will retry");
                markers = null;
                return;
            }
            markers.Sort((a, b) => string.CompareOrdinal(a.name, b.name));

            // classes.txt from the markers present: class id -> label.
            var classes = new SortedDictionary<int, string>();
            foreach (var marker in markers)
            {
                classes[ExtractClass(marker.name)] = ExtractLabel(marker.name);
            }
            var builder = new StringBuilder();
            foreach (var pair in classes)
            {
                builder.AppendLine(pair.Value);
            }
            File.WriteAllText($"{outputDirectory}/classes.txt", builder.ToString());
        }

        // Jitters the capture position around its authored base: a random
        // per-axis offset, re-aimed at the RobotBounds cube's center. The
        // offset rides along even when the authored distance is too close
        // for the cube: the camera then dollies out along its view
        // direction with the offset still applied, so every frame carries
        // randomness. Only if the full offset cannot fit even at 50 m is
        // it shrunk back at the authored distance.
        private void JitterCamera()
        {
            if (positionJitter <= 0f)
            {
                return;
            }
            if (robotBounds == null)
            {
                robotBounds = UnityEngine.Object.FindAnyObjectByType<RobotBounds>();
                if (robotBounds == null)
                {
                    if (!warnedNoBounds)
                    {
                        warnedNoBounds = true;
                        Debug.LogWarning(
                            "[RigStateCapture] no RobotBounds cube in the scene - no camera jitter");
                    }
                    return;
                }
            }
            var bounds = robotBounds.Bounds;
            var offset = new Vector3(
                (float)(jitterRandom.NextDouble() * 2 - 1),
                (float)(jitterRandom.NextDouble() * 2 - 1),
                (float)(jitterRandom.NextDouble() * 2 - 1)) * positionJitter;
            var transform = captureCamera.transform;
            var center = bounds.center;
            var toCamera = cameraBasePosition - center;
            var direction = toCamera.sqrMagnitude > 1e-8f
                ? toCamera.normalized
                : Vector3.back;
            var distance = toCamera.magnitude;
            for (; distance < 50f; distance *= 1.25f)
            {
                var position = center + direction * distance + offset;
                position.y = Mathf.Max(position.y, minCameraHeight);
                transform.position = position;
                transform.LookAt(center);
                if (FitsInFrustum(bounds))
                {
                    Debug.Log(
                        $"[RigStateCapture] jitter: cam {transform.position:F3} " +
                        $"(offset {offset:F2}, dist {distance:F2} m)");
                    return;
                }
            }
            // The full offset cannot fit even at 50 m: shrink it at the
            // authored distance.
            for (var scale = 1f; scale > 0.05f; scale *= 0.5f)
            {
                var position = cameraBasePosition + offset * scale;
                position.y = Mathf.Max(position.y, minCameraHeight);
                transform.position = position;
                transform.LookAt(center);
                if (FitsInFrustum(bounds))
                {
                    return;
                }
            }
            transform.position = cameraBasePosition;
            transform.LookAt(center);
        }

        // True when all eight corners of the bounds project inside the
        // camera's viewport.
        private bool FitsInFrustum(Bounds bounds)
        {
            var ext = bounds.extents;
            for (var i = 0; i < 8; i++)
            {
                var corner = bounds.center + new Vector3(
                    (i & 1) == 0 ? -ext.x : ext.x,
                    (i & 2) == 0 ? -ext.y : ext.y,
                    (i & 4) == 0 ? -ext.z : ext.z);
                var viewport = captureCamera.WorldToViewportPoint(corner);
                if (viewport.z <= 0f || viewport.x < 0f || viewport.x > 1f ||
                    viewport.y < 0f || viewport.y > 1f)
                {
                    return false;
                }
            }
            return true;
        }

        private void WriteLabel(string name)
        {
            EnsureMarkers();
            if (markers == null || markers.Count == 0)
            {
                return;   // no label source yet - retried on the next capture
            }
            var builder = new StringBuilder();
            foreach (var marker in markers)
            {
                if (!TryGetViewportBounds(marker, captureCamera, out var bounds))
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
            File.WriteAllText($"{outputDirectory}/{name}.txt", builder.ToString());
        }

        // Projects the marker quad's four corners through the camera and
        // returns the normalized viewport AABB, or false when any corner
        // is behind the camera plane (marker not fully visible).
        private static bool TryGetViewportCorners(
            Transform marker, Camera camera, out Vector2[] corners)
        {
            corners = new Vector2[QuadCorners.Length];
            for (var i = 0; i < QuadCorners.Length; i++)
            {
                var viewport = camera.WorldToViewportPoint(
                    marker.TransformPoint(QuadCorners[i]));
                if (viewport.z <= 0f)
                {
                    return false;
                }
                corners[i] = new Vector2(viewport.x, viewport.y);
            }
            return true;
        }

        private static bool TryGetViewportBounds(Transform marker, Camera camera, out Bounds bounds)
        {
            bounds = new Bounds();
            if (!TryGetViewportCorners(marker, camera, out var corners))
            {
                return false;
            }
            var min = new Vector2(float.MaxValue, float.MaxValue);
            var max = new Vector2(float.MinValue, float.MinValue);
            foreach (var point in corners)
            {
                min = Vector2.Min(min, point);
                max = Vector2.Max(max, point);
            }
            bounds.SetMinMax(min, max);
            return true;
        }

        // Marker_<class>_<Label> in ascending class order.
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

        private static string ExtractLabel(string markerName)
        {
            var start = "Marker_".Length;
            var end = markerName.IndexOf('_', start);
            return end < 0 ? markerName[start..] : markerName[(end + 1)..];
        }

        private static float Clamp01(float value)
        {
            return Mathf.Clamp(value, 0f, 1f);
        }
    }
}
