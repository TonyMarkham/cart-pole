using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One set's run: draw states until the set's state count is
    // captured. The set's fixed-pose frames come first: capture i takes
    // fixed pose i mod the pose count while i is below FixedPoseFrames,
    // and a random rig state after. Each state is a synchronous
    // main-thread chunk - apply the environment (desk, skybox, light -
    // absolute ranges), drive the rig through the drawn state (rig-local
    // ranges) or the fixed pose and its props, place the
    // camera at its authored pose plus the drawn offsets, compensate the frustum,
    // render the greyscale capture frame, run the 8K verification
    // gate, and write the gated artifacts - all before the next draw.
    // Between states the run awaits, so the player loop and the editor
    // UI keep running: the process never reads as unresponsive. A draw
    // whose frustum compensation exceeds the set's Max Z is not
    // captured and does not count toward States; a discarded fixed-pose
    // frame is retried with the same pose and a fresh environment and
    // camera, so the sequence stays in step. A set with no fixed-pose
    // frames makes the same
    // random draws in the same order as before they existed, so its
    // seed replays the same dataset.
    internal sealed class GenerationSetRunner
    {
        // The wholesale-failure breaker: this many consecutive frames
        // that proposed markers but confirmed none is the detector-
        // broken signature (the retired Python checker's 0/80 run -
        // the motivating never-wipe failure). The run aborts instead
        // of writing an all-empty dataset; partial output stays on
        // disk and the error names it.
        private const int MaxConsecutiveUnconfirmed = 20;

        private const int JpegQuality = GreyscaleEncoder.Quality;

        private readonly GenerationSetSO set;
        private readonly RigSO rig;
        private readonly GameObject assemblyInstance;
        private readonly Camera captureCamera;
        private readonly Light directionalLight;
        private readonly RobotBounds robotBounds;
        private readonly bool keepBoundsInFrustum;
        private readonly Vector3 cameraBasePosition;
        private readonly Quaternion cameraBaseRotation;

        private System.Random random;
        private List<FixedPoseSO> fixedPoses;
        private int fixedPoseFrames;
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
            Vector3 cameraBasePosition,
            Quaternion cameraBaseRotation)
        {
            this.set = set;
            this.rig = rig;
            this.assemblyInstance = assemblyInstance;
            this.captureCamera = captureCamera;
            this.directionalLight = directionalLight;
            this.robotBounds = robotBounds;
            this.keepBoundsInFrustum = keepBoundsInFrustum;
            this.cameraBasePosition = cameraBasePosition;
            this.cameraBaseRotation = cameraBaseRotation;
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
            fixedPoses = ResolveFixedPoses();
            fixedPoseFrames = ResolveFixedPoseFrames();
            Debug.Log(
                $"[GenerationSetRunner] '{set.name}': {set.States} states " +
                $"({fixedPoseFrames} fixed-pose frames first, over {fixedPoses.Count} poses), " +
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
                var fixedPose = captures < fixedPoseFrames
                    ? fixedPoses[captures % fixedPoses.Count]
                    : null;
                if (DrawAndCapture(environment, fixedPose))
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
        // written, does not count toward States). With a fixed pose the
        // rig takes that pose instead of a random state, and the pose's
        // props exist for this draw only. The draw order - environment,
        // rig, camera - is the random sequence a seed replays.
        private bool DrawAndCapture(EnvironmentRandomizer environment, FixedPoseSO fixedPose)
        {
            environment.Apply();
            var props = new List<GameObject>();
            if (fixedPose != null)
            {
                rig.ApplyState(assemblyInstance, fixedPose.CartPosition, fixedPose.PoleRadians);
                foreach (var propPose in fixedPose.Props)
                {
                    var prop = propPose != null ? propPose.Place(assemblyInstance) : null;
                    if (prop != null)
                    {
                        props.Add(prop);
                    }
                }
            }
            else
            {
                rig.ApplyState(
                    assemblyInstance,
                    Draw(rig.CartPositionRange),
                    Draw(rig.PoleRotationRange));
            }
            try
            {
                return PlaceCameraAndCapture();
            }
            finally
            {
                // Destroy is deferred to the end of the frame; inactive
                // now, the props can never reach the next draw's renders.
                foreach (var prop in props)
                {
                    prop.SetActive(false);
                    Object.Destroy(prop);
                }
            }
        }

        // The camera draw, the frustum fit, and the render, gate and
        // write of one frame; false means the Max Z cap discarded it.
        // Each enabled range is an offset from the camera's authored
        // pose: lateral along world X and height along world Y in
        // meters, pitch and yaw in degrees about the authored
        // orientation. A disabled range adds nothing and draws nothing.
        private bool PlaceCameraAndCapture()
        {
            var lateral = set.RandomizeCameraLateral ? Draw(set.CameraLateralRange) : 0f;
            var height = set.RandomizeCameraHeight ? Draw(set.CameraHeightRange) : 0f;
            var pitch = set.RandomizeCameraPitch ? Draw(set.CameraPitchRange) : 0f;
            var yaw = set.RandomizeCameraYaw ? Draw(set.CameraYawRange) : 0f;
            var transform = captureCamera.transform;
            transform.SetPositionAndRotation(
                cameraBasePosition + new Vector3(lateral, height, 0f),
                cameraBaseRotation * Quaternion.Euler(pitch, yaw, 0f));
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
            writer.WriteFrame(name, GreyscaleEncoder.Encode(captureReadback), lines);
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

        // The set's fixed poses without empty list entries, each skipped
        // entry named once.
        private List<FixedPoseSO> ResolveFixedPoses()
        {
            var resolved = new List<FixedPoseSO>();
            foreach (var pose in set.FixedPoses)
            {
                if (pose == null)
                {
                    Debug.LogWarning(
                        $"[GenerationSetRunner] '{set.name}': an empty fixed pose entry is skipped");
                    continue;
                }
                resolved.Add(pose);
            }
            return resolved;
        }

        // How many of the run's frames take a fixed pose: FixedPoseFrames,
        // capped at States, and none without a pose to take. Each
        // mismatch between the count and the list is named, because the
        // run would otherwise quietly make a different dataset.
        private int ResolveFixedPoseFrames()
        {
            var frames = set.FixedPoseFrames;
            if (frames > 0 && fixedPoses.Count == 0)
            {
                Debug.LogWarning(
                    $"[GenerationSetRunner] '{set.name}': {frames} fixed-pose frames but no " +
                    "fixed poses - every frame draws a random rig state");
                return 0;
            }
            if (frames <= 0 && fixedPoses.Count > 0)
            {
                Debug.LogWarning(
                    $"[GenerationSetRunner] '{set.name}': {fixedPoses.Count} fixed poses but " +
                    "0 fixed-pose frames - none of them run");
                return 0;
            }
            if (frames > set.States)
            {
                Debug.LogWarning(
                    $"[GenerationSetRunner] '{set.name}': {frames} fixed-pose frames but " +
                    $"{set.States} States - only {set.States} run");
                return set.States;
            }
            return Mathf.Max(0, frames);
        }

        private float Draw(float2 range)
        {
            return range.x + (float)random.NextDouble() * (range.y - range.x);
        }
    }
}
