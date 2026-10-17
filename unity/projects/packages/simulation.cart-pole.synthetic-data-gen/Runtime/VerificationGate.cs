using System;
using System.Collections.Generic;

using Greyhound.Vision;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // The verification gate - the never-wipe rule's carrier, write-once
    // form. Per captured frame an 8K greyscale render (7680x4320 via 4x
    // supersampling of the capture view - the empirically-confirmed
    // sweet spot where OpenCV's ArUco reliably found all markers) is
    // taken through a telephoto twin of the capture camera, cut into
    // per-marker crops in memory, and each crop goes through the native
    // ghcv detector. A class is confirmed only when its crop decodes to
    // exactly that class's tag - the crop frames one marker, so several
    // ids or a different single id are both an unverifiable box. No
    // crop file is ever written: the instruments leave no trace.
    internal sealed class VerificationGate : IDisposable
    {
        private const int Width8K = 7680;
        private const int Height8K = 4320;

        // The crop's pixel padding around the projected quad, in 8K
        // pixels - detection needs the marker's quiet zone, not a
        // bounding-box-tight image.
        private const int CropPadding = 20;

        private readonly Camera markerCamera;
        private readonly RenderTexture target8K;
        private readonly Texture2D readback8K;
        private bool disposed;

        internal VerificationGate(Camera captureCamera)
        {
            var go = new GameObject("MarkerCamera")
            {
                hideFlags = HideFlags.HideAndDontSave,
            };
            go.transform.SetParent(captureCamera.transform, false);
            markerCamera = go.AddComponent<Camera>();
            markerCamera.CopyFrom(captureCamera);
            markerCamera.enabled = false;
            // The 8K target stays attached for the camera's whole life -
            // its renders and its crop-cut projections share one
            // configuration, so the projection is never re-interpreted
            // through a different aspect.
            target8K = new RenderTexture(Width8K, Height8K, 24);
            markerCamera.targetTexture = target8K;
            readback8K = new Texture2D(
                Width8K, Height8K, TextureFormat.RGB24, false);
        }

        // Renders the 8K twin of the capture camera's current view. The
        // capture camera must already be in its final pose for this
        // draw (including the frustum compensation).
        internal void BeginFrame(Camera captureCamera)
        {
            markerCamera.transform.SetParent(
                captureCamera.transform, false);
            markerCamera.transform.localPosition = Vector3.zero;
            markerCamera.transform.localRotation = Quaternion.identity;
            markerCamera.fieldOfView = captureCamera.fieldOfView;
            markerCamera.Render();
            var previousActive = RenderTexture.active;
            RenderTexture.active = target8K;
            readback8K.ReadPixels(
                new Rect(0, 0, Width8K, Height8K), 0, 0);
            readback8K.Apply();
            RenderTexture.active = previousActive;
        }

        // One pass over the frame's markers: cut each fully visible
        // marker's crop from the 8K readback, detect on the encoded
        // bytes, and return the class ids whose crops confirmed. The
        // caller filters its projected label lines down to these.
        internal HashSet<int> Confirm(
            IReadOnlyList<Transform> markers,
            IReadOnlyDictionary<int, int> classTags,
            int jpegQuality)
        {
            var confirmed = new HashSet<int>();
            foreach (var marker in markers)
            {
                if (!TryCropRect(marker, out var x0, out var y0, out var w, out var h))
                {
                    continue;
                }
                var classId = ExtractClass(marker.name);
                if (!classTags.TryGetValue(classId, out var tagId))
                {
                    continue;
                }
                var crop = new Texture2D(w, h, TextureFormat.RGB24, false);
                crop.SetPixels(readback8K.GetPixels(x0, y0, w, h));
                crop.Apply();
                var jpeg = crop.EncodeToJPG(jpegQuality);
                UnityEngine.Object.Destroy(crop);
                int[] detected = Ghcv.Detect(jpeg, out string error);
                if (error != null)
                {
                    Debug.LogWarning(
                        $"[VerificationGate] {marker.name}: {error}");
                    continue;
                }
                if (detected.Length == 1 && detected[0] == tagId)
                {
                    confirmed.Add(classId);
                }
            }
            return confirmed;
        }

        // The crop's pixel rectangle from the exact projected quad
        // corners, padded and clamped into the 8K image - no detection
        // involved in the cutting. The y flip mirrors the readback's
        // bottom-up origin.
        private bool TryCropRect(
            Transform marker, out int x0, out int y0, out int w, out int h)
        {
            x0 = y0 = w = h = 0;
            var min = new Vector2(float.MaxValue, float.MaxValue);
            var max = new Vector2(float.MinValue, float.MinValue);
            foreach (var corner in QuadCorners)
            {
                var viewport = markerCamera.WorldToViewportPoint(
                    marker.TransformPoint(corner));
                if (viewport.z <= 0f)
                {
                    return false;
                }
                min = Vector2.Min(min, new Vector2(viewport.x, viewport.y));
                max = Vector2.Max(max, new Vector2(viewport.x, viewport.y));
            }
            x0 = Mathf.Clamp(
                Mathf.FloorToInt(min.x * Width8K) - CropPadding,
                0, Width8K - 1);
            var yTop = Mathf.Clamp(
                Mathf.FloorToInt((1f - max.y) * Height8K) - CropPadding,
                0, Height8K - 1);
            w = Mathf.Clamp(
                Mathf.CeilToInt((max.x - min.x) * Width8K) + 2 * CropPadding,
                1, Width8K - x0);
            h = Mathf.Clamp(
                Mathf.CeilToInt((max.y - min.y) * Height8K) + 2 * CropPadding,
                1, Height8K - yTop);
            y0 = Height8K - yTop - h;
            return true;
        }

        // The marker prefab's quad corners in the quad's local space.
        private static readonly Vector3[] QuadCorners =
        {
            new(-0.5f, -0.5f, 0f),
            new(0.5f, -0.5f, 0f),
            new(0.5f, 0.5f, 0f),
            new(-0.5f, 0.5f, 0f),
        };

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

        public void Dispose()
        {
            if (disposed)
            {
                return;
            }
            disposed = true;
            if (markerCamera != null)
            {
                UnityEngine.Object.Destroy(markerCamera.gameObject);
            }
            if (target8K != null)
            {
                UnityEngine.Object.Destroy(target8K);
            }
            if (readback8K != null)
            {
                UnityEngine.Object.Destroy(readback8K);
            }
        }
    }
}
