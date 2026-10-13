using System;
using System.IO;
using UnityEngine;
using UnityEngine.InputSystem;
using UnityEngine.Rendering;
using UnityEngine.Rendering.Universal;

namespace CartPole.Vision
{
    // Captures a PNG snapshot from the assigned camera (the one configured to
    // mimic the physical cam) and saves it to disk. Capture from the inspector
    // via the context menu, in Play mode via the configured hotkey, or from
    // code through Capture(). Supersampling smooths texture boundaries as well
    // as geometry, which MSAA alone cannot do for the point-filtered markers.
    public sealed class SnapshotCapture : MonoBehaviour
    {
        public Camera captureCamera;

        public Key hotkey = Key.F12;

        [Min(1)] public int captureWidth = 960;

        [Min(1)] public int captureHeight = 540;

        [Tooltip("Render at 1x, 2x, or 4x the output dimensions, then average down to the PNG size.")]
        public int supersampling = 2;

        // Additional geometry AA, where supported by the GPU and renderer.
        // This does not antialias boundaries inside marker textures.
        [Tooltip("MSAA samples: 1, 2, 4, or 8. Supersampling works independently of MSAA.")]
        public int msaaSamples = 4;

        // Output folder for the PNGs; empty means <project>/Snapshots in the
        // editor and persistent data in builds.
        public string outputFolder = "";

        private void Update()
        {
            var keyboard = Keyboard.current;
            if (keyboard != null && keyboard[hotkey].wasPressedThisFrame)
            {
                Capture();
            }
        }

        [ContextMenu("Capture Snapshot")]
        public void Capture()
        {
            if (captureCamera == null)
            {
                Debug.LogWarning(
                    "[SnapshotCapture] no camera assigned - not falling back to " +
                    "Camera.main so the captured camera is never ambiguous");
                return;
            }
            if (captureWidth < 1 || captureHeight < 1 ||
                (supersampling != 1 && supersampling != 2 && supersampling != 4) ||
                (msaaSamples != 1 && msaaSamples != 2 && msaaSamples != 4 && msaaSamples != 8))
            {
                Debug.LogWarning("[SnapshotCapture] use positive dimensions, supersampling 1/2/4, and MSAA 1/2/4/8");
                return;
            }
            if ((long)captureWidth * supersampling > SystemInfo.maxTextureSize ||
                (long)captureHeight * supersampling > SystemInfo.maxTextureSize)
            {
                Debug.LogWarning($"[SnapshotCapture] supersampled dimensions exceed the GPU texture limit ({SystemInfo.maxTextureSize})");
                return;
            }

            var folder = ResolveFolder();
            Directory.CreateDirectory(folder);

            var previousTarget = captureCamera.targetTexture;
            var previousActive = RenderTexture.active;
            var previousSrgbWrite = GL.sRGBWrite;
            captureCamera.TryGetComponent<UniversalAdditionalCameraData>(out var cameraData);
            var previousAA = cameraData != null ? cameraData.antialiasing : AntialiasingMode.None;
            RenderTexture target = null;
            RenderTexture halfSize = null;
            RenderTexture output = null;
            Texture2D readback = null;
            try
            {
                // Allocate on demand so the context menu also works outside Play
                // mode and Inspector changes take effect on the next capture.
                var descriptor = new RenderTextureDescriptor(
                    captureWidth * supersampling, captureHeight * supersampling,
                    RenderTextureFormat.ARGB32, 24)
                {
                    sRGB = QualitySettings.activeColorSpace == ColorSpace.Linear,
                    msaaSamples = msaaSamples,
                    useMipMap = false,
                    autoGenerateMips = false
                };
                descriptor.msaaSamples = SystemInfo.GetRenderTextureSupportedMSAASampleCount(descriptor);
                target = RenderTexture.GetTemporary(descriptor);
                target.filterMode = FilterMode.Bilinear;

                // A standalone snapshot has no valid temporal history at this
                // resolution. Supersampling provides spatial AA instead.
                if (cameraData != null && previousAA == AntialiasingMode.TemporalAntiAliasing)
                {
                    cameraData.antialiasing = AntialiasingMode.None;
                }
                RenderCamera(target);

                GL.sRGBWrite = QualitySettings.activeColorSpace == ColorSpace.Linear;
                var source = target;
                if (supersampling == 4)
                {
                    halfSize = GetDownsampleTarget(captureWidth * 2, captureHeight * 2);
                    Graphics.Blit(source, halfSize);
                    source = halfSize;
                }
                // Each 2:1 bilinear pass averages a 2x2 block. A single 4:1
                // blit would skip samples. The 1x path also resolves MSAA into
                // a single-sample texture before CPU readback.
                output = GetDownsampleTarget(captureWidth, captureHeight);
                Graphics.Blit(source, output);
                RenderTexture.active = output;
                readback = new Texture2D(captureWidth, captureHeight, TextureFormat.RGB24, false);
                readback.ReadPixels(new Rect(0, 0, captureWidth, captureHeight), 0, 0, false);

                var stamp = DateTime.Now.ToString("yyyyMMdd-HHmmss-fff");
                var path = Path.Combine(folder, $"snapshot-{stamp}.png");
                File.WriteAllBytes(path, readback.EncodeToPNG());
                Debug.Log($"[SnapshotCapture] saved {path} ({captureWidth}x{captureHeight}, " +
                    $"rendered {target.width}x{target.height} from '{captureCamera.name}')");
            }
            finally
            {
                captureCamera.targetTexture = previousTarget;
                if (cameraData != null)
                {
                    cameraData.antialiasing = previousAA;
                }
                RenderTexture.active = previousActive;
                GL.sRGBWrite = previousSrgbWrite;
                if (output != null) RenderTexture.ReleaseTemporary(output);
                if (halfSize != null) RenderTexture.ReleaseTemporary(halfSize);
                if (target != null) RenderTexture.ReleaseTemporary(target);
                if (readback != null)
                {
                    if (Application.isPlaying) Destroy(readback);
                    else DestroyImmediate(readback);
                }
            }
        }

        private void RenderCamera(RenderTexture destination)
        {
            if (GraphicsSettings.currentRenderPipeline == null)
            {
                captureCamera.targetTexture = destination;
                captureCamera.Render();
                return;
            }

            var request = new UniversalRenderPipeline.SingleCameraRequest { destination = destination };
            if (!RenderPipeline.SupportsRenderRequest(captureCamera, request))
            {
                throw new InvalidOperationException("[SnapshotCapture] the active pipeline does not support a URP single-camera render request");
            }
            RenderPipeline.SubmitRenderRequest(captureCamera, request);
        }

        private static RenderTexture GetDownsampleTarget(int width, int height)
        {
            var texture = RenderTexture.GetTemporary(width, height, 0,
                RenderTextureFormat.ARGB32, RenderTextureReadWrite.sRGB);
            texture.filterMode = FilterMode.Bilinear;
            texture.wrapMode = TextureWrapMode.Clamp;
            return texture;
        }

        private string ResolveFolder()
        {
            if (!string.IsNullOrEmpty(outputFolder))
            {
                return outputFolder;
            }
#if UNITY_EDITOR
            return Path.Combine(Directory.GetParent(Application.dataPath).FullName, "Snapshots");
#else
            return Path.Combine(Application.persistentDataPath, "Snapshots");
#endif
        }
    }
}
