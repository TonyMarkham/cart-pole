using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

using Greyhound.MuJoCo;

using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One set's run: draw states until the set's state count is
    // captured. The set's fixed-pose frames come first: capture i takes
    // fixed pose i mod the pose count while i is below FixedPoseFrames,
    // and a random state after. Each state is a synchronous
    // main-thread chunk - apply the environment (desk, skybox, light -
    // absolute ranges), pose the robot's joints at the drawn state (each
    // joint's own range, the set's UnlimitedHinge for an unlimited
    // hinge) or the fixed pose and its props, place the
    // camera at its authored pose plus the drawn offsets, slide it in X and Y
    // until the robot bounds are in the frame,
    // render the greyscale capture frame, run the 8K verification
    // gate, and write the gated artifacts - all before the next draw.
    // Between states the run awaits, so the player loop and the editor
    // UI keep running: the process never reads as unresponsive. A draw
    // that no X/Y slide fits the robot bounds in the frame is not
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

        // ResolveBounds' passes. With the camera turned at most 5 degrees
        // from the authored pose, a world-X slide changes each corner's
        // depth by at most sin 5 deg = 0.087 of the slide, so each pass
        // leaves under a tenth of the error before it: 8 passes is far
        // past half a pixel. Hitting the limit discards the draw.
        private const int k_MaxBoundsPasses = 8;

        // ResolveBounds gives up when the camera's right (up) axis is turned
        // this far from world X (Y): cos 60 deg. The authored pose looks
        // down world +Z, so a real draw is never close to it.
        private const float k_MinAxisAlignment = 0.5f;

        private readonly ResolvedGenerationSet set;
        // The markers MarkerManager placed, class i at index i.
        private readonly IReadOnlyList<PlacedMarker> placedMarkers;
        private readonly IReadOnlyList<JointDriver> drivers;
        private readonly GameObject assemblyInstance;
        private readonly Camera captureCamera;
        private readonly Light directionalLight;
        private readonly RobotBounds robotBounds;
        private readonly bool keepBoundsInFrustum;
        private readonly Vector3 cameraBasePosition;
        private readonly Quaternion cameraBaseRotation;

        // RobotBounds' four corners, set by RunAsync when the bounds are
        // kept in the frame.
        private Transform[] boundsCorners;

        private System.Random random;
        private IReadOnlyList<ResolvedFixedPose> fixedPoses;
        private int fixedPoseFrames;
        private DatasetWriter writer;
        private VerificationGate gate;
        private RenderTexture captureTarget;
        private Texture2D captureReadback;
        private int consecutiveUnconfirmed;

        // The run folder this set wrote, null until RunAsync starts: the
        // generator reports it back to the client that sent the job.
        internal string RunDirectory => writer?.RunDirectory;

        // The frames this set wrote: 0 when RunAsync returned before
        // capturing (no markers placed).
        internal int Frames => writer?.Frames ?? 0;

        // Why the run stopped before capturing all States frames, null
        // when it finished: no markers placed, a RobotBounds corner
        // missing, the bounds fit rejecting
        // nearly every draw, or the gate confirming nothing.
        internal string Failure { get; private set; }

        internal GenerationSetRunner(
            ResolvedGenerationSet set,
            IReadOnlyList<PlacedMarker> placedMarkers,
            IReadOnlyList<JointDriver> drivers,
            GameObject assemblyInstance,
            Camera captureCamera,
            Light directionalLight,
            RobotBounds robotBounds,
            bool keepBoundsInFrustum,
            Vector3 cameraBasePosition,
            Quaternion cameraBaseRotation)
        {
            this.set = set;
            this.placedMarkers = placedMarkers;
            this.drivers = drivers;
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
            writer = new DatasetWriter(set.OutputDirectory, set.Name);
            if (placedMarkers.Count == 0)
            {
                // Zero markers is run-fatal: every frame would be an
                // all-empty label file.
                Failure = "the MarkerManager placed no markers; the set was skipped";
                Debug.LogError($"[GenerationSetRunner] {Failure}");
                return;
            }
            if (keepBoundsInFrustum)
            {
                boundsCorners = new[]
                {
                    robotBounds.TopLeft,
                    robotBounds.TopRight,
                    robotBounds.BottomLeft,
                    robotBounds.BottomRight,
                };
                if (boundsCorners.Any(corner => corner == null))
                {
                    Failure =
                        $"RobotBounds '{robotBounds.name}' is missing a corner " +
                        "(Top Left, Top Right, Bottom Left, Bottom Right); the set was skipped";
                    Debug.LogError($"[GenerationSetRunner] {Failure}");
                    return;
                }
            }
            writer.WriteMarkersJson(placedMarkers);
            captureTarget = new RenderTexture(
                set.CameraImageWidth, set.CameraImageHeight, 24);
            captureReadback = new Texture2D(
                set.CameraImageWidth, set.CameraImageHeight,
                TextureFormat.RGB24, false);
            gate = new VerificationGate(captureCamera);
            var environment = new EnvironmentRandomizer(
                set, random, directionalLight);
            fixedPoses = set.FixedPoses;
            fixedPoseFrames = ResolveFixedPoseFrames();
            Debug.Log(
                $"[GenerationSetRunner] '{set.Name}': {set.States} states " +
                $"({fixedPoseFrames} fixed-pose frames first, over {fixedPoses.Count} poses), " +
                $"seed {set.Seed} -> {writer.RunDirectory}");
            var captures = 0;
            var attempts = 0;
            var discarded = 0;
            while (captures < set.States)
            {
                attempts++;
                // The redraw guard: a draw only repeats on a bounds-fit
                // discard, so this cap can only trip on a pathological
                // configuration (bounds too big for the frame at every pose the
                // ranges can draw) - stop with the partial output
                // named rather than spin forever.
                if (attempts > set.States * 20 + 100)
                {
                    Failure =
                        $"'{set.Name}': {attempts} draws " +
                        $"produced only {captures} captures - the bounds fit " +
                        "rejects nearly every draw; stopping with the " +
                        $"partial run at {writer.RunDirectory}";
                    Debug.LogError($"[GenerationSetRunner] {Failure}");
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
                    Failure =
                        $"'{set.Name}': " +
                        $"{MaxConsecutiveUnconfirmed} consecutive frames proposed " +
                        "markers but the gate confirmed none - the detector " +
                        "looks broken; aborting the set before it writes an " +
                        $"all-empty dataset (partial run at {writer.RunDirectory})";
                    Debug.LogError($"[GenerationSetRunner] {Failure}");
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
                $"[GenerationSetRunner] '{set.Name}': {writer.Frames} frames, " +
                $"{discarded} draws discarded by the bounds fit, " +
                $"elapsed {stopwatch.Elapsed:hh\\:mm\\:ss}");
        }

        // One draw through the whole pipeline; false means the draw was
        // discarded by the bounds fit (nothing rendered, nothing
        // written, does not count toward States). With a fixed pose the
        // robot takes that pose instead of a random state, and the pose's
        // props exist for this draw only. The draw order - environment,
        // joints in the Root's order, camera - is the random sequence a
        // seed replays.
        private bool DrawAndCapture(EnvironmentRandomizer environment, ResolvedFixedPose fixedPose)
        {
            environment.Apply();
            var props = new List<GameObject>();
            if (fixedPose != null)
            {
                fixedPose.Apply(drivers);
                foreach (var propPose in fixedPose.Props)
                {
                    var prop = propPose.Place(assemblyInstance);
                    if (prop != null)
                    {
                        props.Add(prop);
                    }
                }
            }
            else
            {
                foreach (var driver in drivers)
                {
                    if (driver.TryGetRange(out var lo, out var hi))
                    {
                        driver.Apply(Draw(new float2(lo, hi)));
                    }
                    else if (driver.Type == JointType.Hinge && set.UnlimitedHinge.HasValue)
                    {
                        driver.Apply(Draw(set.UnlimitedHinge.Value));
                    }
                    else
                    {
                        driver.ResetPose();
                    }
                }
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

        // The camera draw, the bounds fit, and the render, gate and
        // write of one frame; false means the bounds fit discarded it.
        // Each range the set has is an offset from the camera's authored
        // pose: lateral along world X, height along world Y and distance
        // along world Z in meters, pitch, yaw and roll in degrees about the
        // authored orientation. A range the set leaves out adds nothing and
        // draws nothing. Distance and roll are drawn after the four older
        // ranges, so a set without them draws the same sequence as before.
        private bool PlaceCameraAndCapture()
        {
            var lateral = DrawOrZero(set.CameraLateral);
            var height = DrawOrZero(set.CameraHeight);
            var pitch = DrawOrZero(set.CameraPitch);
            var yaw = DrawOrZero(set.CameraYaw);
            var distance = DrawOrZero(set.CameraDistance);
            var roll = DrawOrZero(set.CameraRoll);
            var transform = captureCamera.transform;
            transform.SetPositionAndRotation(
                cameraBasePosition + new Vector3(lateral, height, distance),
                cameraBaseRotation * Quaternion.Euler(pitch, yaw, roll));
            // The camera renders into the capture image from here to the
            // label projection: the bounds fit measures the frame the image
            // will have (the render-configuration rule).
            var previousTarget = captureCamera.targetTexture;
            captureCamera.targetTexture = captureTarget;
            if (keepBoundsInFrustum && !ResolveBounds())
            {
                captureCamera.targetTexture = previousTarget;
                Debug.Log(
                    $"[GenerationSetRunner] '{set.Name}': draw discarded - " +
                    "no X/Y slide of the camera fits the robot bounds in the frame");
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
            captureCamera.Render();
            // The label projection runs while the camera is still in
            // the configuration that rendered the image (the render-
            // configuration rule) - the lines are proposals until the
            // gate confirms them.
            var proposed = LabelProjector.Project(placedMarkers, captureCamera);
            captureCamera.targetTexture = previousTarget;
            RenderTexture.active = captureTarget;
            captureReadback.ReadPixels(
                new Rect(0, 0, set.CameraImageWidth, set.CameraImageHeight), 0, 0);
            captureReadback.Apply();
            RenderTexture.active = previousActive;

            gate.BeginFrame(captureCamera);
            var confirmed = gate.Confirm(placedMarkers, JpegQuality);
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
                .Where(entry => confirmed.Contains(entry.ClassId))
                .Select(entry => entry.Line)
                .ToList();
            writer.WriteFrame(name, GreyscaleEncoder.Encode(captureReadback), lines);
            return true;
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
                    $"[GenerationSetRunner] '{set.Name}': {frames} fixed-pose frames but no " +
                    "fixed poses - every frame draws a random state");
                return 0;
            }
            if (frames <= 0 && fixedPoses.Count > 0)
            {
                Debug.LogWarning(
                    $"[GenerationSetRunner] '{set.Name}': {fixedPoses.Count} fixed poses but " +
                    "0 fixed-pose frames - none of them run");
                return 0;
            }
            if (frames > set.States)
            {
                Debug.LogWarning(
                    $"[GenerationSetRunner] '{set.Name}': {frames} fixed-pose frames but " +
                    $"{set.States} States - only {set.States} run");
                return set.States;
            }
            return Mathf.Max(0, frames);
        }

        // The visible frame's size in metres (x width, y height) at `depth`
        // metres in front of the camera, as the camera is placed now. Call
        // it while the camera renders into the capture image: the capture
        // camera is a physical camera with a gate fit, so the frame's
        // height depends on the image's aspect (the render-configuration
        // rule). The size grows in proportion to the depth: at twice the
        // distance the frame is twice as wide and twice as tall.
        private Vector2 DetermineImageFrame(float depth)
        {
            var bottomLeft = captureCamera.ViewportToWorldPoint(new Vector3(0f, 0f, depth));
            var bottomRight = captureCamera.ViewportToWorldPoint(new Vector3(1f, 0f, depth));
            var topLeft = captureCamera.ViewportToWorldPoint(new Vector3(0f, 1f, depth));
            return new Vector2(
                Vector3.Distance(bottomLeft, bottomRight),
                Vector3.Distance(bottomLeft, topLeft));
        }

        // Slides the camera along world X and world Y, never Z, until the
        // four corners of the robot bounds are all inside the frame. False
        // when no slide can do it: a corner is behind the camera, or the
        // corners span more than the frame is wide or tall at their
        // distance. Call it while the camera renders into the capture
        // image (see DetermineImageFrame).
        //
        // The rule per corner: projected through the camera, a corner lands
        // at a viewport point, 0 to 1 across and up the frame. Sliding the
        // camera s metres along its own right axis moves that point
        // s / (frame width at the corner's depth) to the left; for the
        // point to stay between 0 and 1, s must lie between
        // (x - 1) x width and x x width. Every corner gives such a range;
        // the slide is the value nearest 0 inside all four (no slide when
        // they are already in), and no value exists when the ranges do not
        // overlap. The same holds for up and the frame height. The camera
        // turns at most a few degrees from the authored pose, so world X
        // and Y are close to, not exactly, its right and up axes: the
        // slide is divided by how far world X (Y) points along right (up),
        // and repeated from the new position until the corners are in,
        // each pass removing most of what the last one left.
        private bool ResolveBounds()
        {
            var transform = captureCamera.transform;
            var rightPerWorldX = Vector3.Dot(transform.right, Vector3.right);
            var upPerWorldY = Vector3.Dot(transform.up, Vector3.up);
            if (rightPerWorldX < k_MinAxisAlignment || upPerWorldY < k_MinAxisAlignment)
            {
                return false;
            }
            // A corner within half a pixel of the frame's edge is in.
            var tolerance = new Vector2(
                0.5f / set.CameraImageWidth,
                0.5f / set.CameraImageHeight);
            for (var pass = 0; pass < k_MaxBoundsPasses; pass++)
            {
                var low = new Vector2(float.NegativeInfinity, float.NegativeInfinity);
                var high = new Vector2(float.PositiveInfinity, float.PositiveInfinity);
                var isInside = true;
                foreach (var corner in boundsCorners)
                {
                    var viewport = captureCamera.WorldToViewportPoint(corner.position);
                    if (viewport.z <= 0f)
                    {
                        return false;
                    }
                    isInside &=
                        viewport.x >= -tolerance.x && viewport.x <= 1f + tolerance.x &&
                        viewport.y >= -tolerance.y && viewport.y <= 1f + tolerance.y;
                    var frame = DetermineImageFrame(viewport.z);
                    low.x = Mathf.Max(low.x, (viewport.x - 1f) * frame.x);
                    high.x = Mathf.Min(high.x, viewport.x * frame.x);
                    low.y = Mathf.Max(low.y, (viewport.y - 1f) * frame.y);
                    high.y = Mathf.Min(high.y, viewport.y * frame.y);
                }
                if (isInside)
                {
                    return true;
                }
                if (low.x > high.x || low.y > high.y)
                {
                    return false;
                }
                var slide = new Vector2(
                    Mathf.Clamp(0f, low.x, high.x),
                    Mathf.Clamp(0f, low.y, high.y));
                transform.position += new Vector3(
                    slide.x / rightPerWorldX,
                    slide.y / upPerWorldY,
                    0f);
            }
            return false;
        }

        private float Draw(float2 range)
        {
            return range.x + (float)random.NextDouble() * (range.y - range.x);
        }

        private float DrawOrZero(float2? range)
        {
            return range.HasValue ? Draw(range.Value) : 0f;
        }
    }
}
