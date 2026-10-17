using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One set's run: draw states until the set's state count is
    // captured. Each state is a synchronous main-thread chunk - apply
    // the environment (desk, skybox, light - absolute ranges), drive
    // the rig through the drawn state (rig-local ranges), place the
    // camera at the drawn absolute pose, compensate the frustum,
    // render the greyscale capture frame, run the 8K verification
    // gate, and write the gated artifacts - all before the next draw.
    // Between states the run awaits, so the player loop and the editor
    // UI keep running: the process never reads as unresponsive. A draw
    // whose frustum compensation exceeds the set's Max Z is not
    // captured and does not count toward States.
    internal sealed class GenerationSetRunner
    {
        // The wholesale-failure breaker: this many consecutive frames
        // that proposed markers but confirmed none is the detector-
        // broken signature (the retired Python checker's 0/80 run -
        // the motivating never-wipe failure). The run aborts instead
        // of writing an all-empty dataset; partial output stays on
        // disk and the error names it.
        private const int MaxConsecutiveUnconfirmed = 20;

        private const int JpegQuality = 90;

        private readonly GenerationSetSO set;
        private readonly RigSO rig;
        private readonly GameObject assemblyInstance;
        private readonly Camera captureCamera;
        private readonly Light directionalLight;
        private readonly RobotBounds robotBounds;
        private readonly bool keepBoundsInFrustum;
        private readonly float cameraBaseZ;

        private System.Random random;
        private DatasetWriter writer;
        private VerificationGate gate;
        private List<Transform> markers;
        private Dictionary<int, int> classTags;
        private RenderTexture captureTarget;
        private Texture2D captureReadback;
        private int consecutiveUnconfirmed;

        internal GenerationSetRunner(
            GenerationSetSO set,
            RigSO rig,
            GameObject assemblyInstance,
            Camera captureCamera,
            Light directionalLight,
            RobotBounds robotBounds,
            bool keepBoundsInFrustum,
            float cameraBaseZ)
        {
            this.set = set;
            this.rig = rig;
            this.assemblyInstance = assemblyInstance;
            this.captureCamera = captureCamera;
            this.directionalLight = directionalLight;
            this.robotBounds = robotBounds;
            this.keepBoundsInFrustum = keepBoundsInFrustum;
            this.cameraBaseZ = cameraBaseZ;
        }

        internal async Task RunAsync()
        {
            var stopwatch = System.Diagnostics.Stopwatch.StartNew();
            random = new System.Random(set.Seed);
            writer = new DatasetWriter(set.OutputDirectory, set.name);
            writer.WriteMarkersJson(rig);
            if (!ResolveMarkers())
            {
                return;
            }
            captureTarget = new RenderTexture(
                set.CameraImageWidth, set.CameraImageHeight, 24);
            captureReadback = new Texture2D(
                set.CameraImageWidth, set.CameraImageHeight,
                TextureFormat.RGB24, false);
            gate = new VerificationGate(captureCamera);
            var environment = new EnvironmentRandomizer(
                set, random, directionalLight);
            Debug.Log(
                $"[GenerationSetRunner] '{set.name}': {set.States} states, " +
                $"seed {set.Seed} -> {writer.RunDirectory}");
            var captures = 0;
            var attempts = 0;
            var discarded = 0;
            while (captures < set.States)
            {
                attempts++;
                // The redraw guard: a draw only repeats on a Max Z
                // discard, so this cap can only trip on a pathological
                // configuration (Max Z too small for every pose the
                // ranges can draw) - stop with the partial output
                // named rather than spin forever.
                if (attempts > set.States * 20 + 100)
                {
                    Debug.LogError(
                        $"[GenerationSetRunner] '{set.name}': {attempts} draws " +
                        $"produced only {captures} captures - the Max Z cap " +
                        "rejects nearly every draw; stopping with the " +
                        $"partial run at {writer.RunDirectory}");
                    break;
                }
                if (DrawAndCapture(environment))
                {
                    captures++;
                }
                else
                {
                    discarded++;
                }
                if (consecutiveUnconfirmed >= MaxConsecutiveUnconfirmed)
                {
                    Debug.LogError(
                        $"[GenerationSetRunner] '{set.name}': " +
                        $"{MaxConsecutiveUnconfirmed} consecutive frames proposed " +
                        "markers but the gate confirmed none - the detector " +
                        "looks broken; aborting the set before it writes an " +
                        $"all-empty dataset (partial run at {writer.RunDirectory})");
                    break;
                }

                // One editor frame between states: the await resumes on
                // the main thread through Unity's synchronization
                // context, so rendering, input and the editor UI run
                // here - the per-state pipeline above is never
                // interleaved with anything else.
                await Task.Yield();
            }
            gate.Dispose();
            environment.Shutdown();
            captureTarget.Release();
            Object.Destroy(captureTarget);
            Object.Destroy(captureReadback);
            Debug.Log(
                $"[GenerationSetRunner] '{set.name}': {writer.Frames} frames, " +
                $"{discarded} draws discarded on the Max Z cap, " +
                $"elapsed {stopwatch.Elapsed:hh\\:mm\\:ss}");
        }

        // One draw through the whole pipeline; false means the draw was
        // discarded on the Max Z cap (nothing rendered, nothing
        // written, does not count toward States).
        private bool DrawAndCapture(EnvironmentRandomizer environment)
        {
            environment.Apply();
            rig.ApplyState(
                assemblyInstance,
                Draw(rig.CartPositionRange),
                Draw(rig.PoleRotationRange));
            var transform = captureCamera.transform;
            transform.SetPositionAndRotation(
                new Vector3(
                    Draw(set.CameraLateralRange),
                    Draw(set.CameraHeightRange),
                    cameraBaseZ),
                Quaternion.Euler(
                    Draw(set.CameraPitchRange),
                    Draw(set.CameraYawRange),
                    0f));
            var bounds = robotBounds.Bounds;
            if (keepBoundsInFrustum && !FrustumFitter.Fit(captureCamera, bounds, set.MaxZ))
            {
                Debug.Log(
                    $"[GenerationSetRunner] '{set.name}': draw discarded - " +
                    "the frustum compensation exceeded the Max Z cap");
                return false;
            }

            // The bounds cube is a real mesh: hidden for the duration
            // of the two renders, restored after, so it never pollutes
            // a frame even while its renderer is on for scene editing.
            var boundsRenderer = robotBounds.GetComponent<MeshRenderer>();
            var boundsWasVisible = boundsRenderer != null && boundsRenderer.enabled;
            if (boundsWasVisible)
            {
                boundsRenderer.enabled = false;
            }

            var name = writer.NextFrameName();
            var previousActive = RenderTexture.active;
            var previousTarget = captureCamera.targetTexture;
            captureCamera.targetTexture = captureTarget;
            captureCamera.Render();
            // The label projection runs while the camera is still in
            // the configuration that rendered the image (the render-
            // configuration rule) - the lines are proposals until the
            // gate confirms them.
            var proposed = LabelProjector.Project(markers, captureCamera, classTags);
            captureCamera.targetTexture = previousTarget;
            RenderTexture.active = captureTarget;
            captureReadback.ReadPixels(
                new Rect(0, 0, set.CameraImageWidth, set.CameraImageHeight), 0, 0);
            captureReadback.Apply();
            RenderTexture.active = previousActive;

            gate.BeginFrame(captureCamera);
            var confirmed = gate.Confirm(markers, classTags, JpegQuality);
            if (boundsWasVisible && boundsRenderer != null)
            {
                boundsRenderer.enabled = true;
            }

            // The wholesale-failure breaker's counter: proposals
            // followed by zero confirmations is the detector-broken
            // signature; a frame with no proposals (all markers off
            // frame) is a legitimate background frame, not a failure.
            if (proposed.Count > 0 && confirmed.Count == 0)
            {
                consecutiveUnconfirmed++;
            }
            else
            {
                consecutiveUnconfirmed = 0;
            }

            var lines = proposed
                .Where(line => confirmed.Contains(int.Parse(line.Split(' ')[0])))
                .ToList();
            writer.WriteFrame(name, GreyscaleJpeg(captureReadback), lines);
            return true;
        }

        // The marker quads and the class-to-tag map, resolved once per
        // set. Zero quads is a run-fatal configuration - the rig's
        // markers were never applied - so the set stops here rather
        // than write an all-empty dataset.
        private bool ResolveMarkers()
        {
            classTags = new Dictionary<int, int>();
            var classIndex = 0;
            foreach (var marker in rig.Markers)
            {
                if (marker != null && marker.TagId >= 0)
                {
                    classTags[classIndex] = marker.TagId;
                }
                classIndex++;
            }
            markers = new List<Transform>();
            foreach (Transform node in
                assemblyInstance.GetComponentsInChildren<Transform>(true))
            {
                if (node.name.StartsWith("Marker_"))
                {
                    markers.Add(node);
                }
            }
            markers.Sort((a, b) => string.CompareOrdinal(a.name, b.name));
            if (markers.Count == 0)
            {
                Debug.LogError(
                    "[GenerationSetRunner] no Marker_* quads under the " +
                    "assembly instance - the rig's markers were never " +
                    "applied; skipping the set");
                return false;
            }
            return true;
        }

        // Greyscale is the only render path: the color readback is
        // luminance-equalized into a neutral image before encoding.
        // The channels stay RGB (equal values, neutral chroma) because
        // EncodeToJPG's single-channel support is not guaranteed
        // across platforms - the image carries no color information
        // either way.
        private static byte[] GreyscaleJpeg(Texture2D source)
        {
            var pixels = source.GetPixels();
            for (var i = 0; i < pixels.Length; i++)
            {
                var l = pixels[i].r * 0.299f +
                    pixels[i].g * 0.587f + pixels[i].b * 0.114f;
                pixels[i] = new Color(l, l, l);
            }
            var grey = new Texture2D(
                source.width, source.height, TextureFormat.RGB24, false);
            grey.SetPixels(pixels);
            grey.Apply();
            var jpeg = grey.EncodeToJPG(JpegQuality);
            Object.Destroy(grey);
            return jpeg;
        }

        private float Draw(float2 range)
        {
            return range.x + (float)random.NextDouble() * (range.y - range.x);
        }
    }
}
