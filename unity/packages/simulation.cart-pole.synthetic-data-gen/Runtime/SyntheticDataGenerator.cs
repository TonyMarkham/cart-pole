using System;
using System.Collections.Generic;
using CartPole.Vision;
using Google.Protobuf;
using Greyhound.MuJoCo;
using Greyhound.MuJoCo.ScriptableObjects;
using UnityEngine;

// ReSharper disable InconsistentNaming
// ReSharper disable UnusedMember.Global

namespace Simulation.CartPole.SyntheticDataGen
{
    // The dataset generator in a Unity player: it renders the labeled frames
    // of one generation set at a time, as the generator (the Rust
    // orchestrator in __synthetic__/generator/) sends them.
    //
    // A job arrives as one binary WebSocket message on the StateServer:
    // a GenerationJob (protobuf, __synthetic__/generator/proto/
    // generation_set.proto) holding the set and the output directory. Its
    // names are looked up first (ResolvedGenerationSet: desks, skyboxes and
    // props in the catalog, fixed-pose joints among the drivers); then the
    // set runs and writes run_<set>_<timestamp>/ under the output directory;
    // then the client that sent the job gets one GenerationDone back, with
    // the run folder and frame count, or the error that stopped it. One job
    // at a time: a job that arrives while another runs is refused with an
    // error, not queued.
    //
    // The joints are posed through the StateServer's JointDrivers: the
    // StateServer attached one per joint of its Root in Awake, before this
    // Start runs, so the generator attaches none.
    public sealed class SyntheticDataGenerator : MonoBehaviour
    {
        // Every log line starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[SyntheticDataGenerator]";

        [Header("Robot")]
        // The robot's StateServer (on the same GameObject in the scene).
        // Its Root's `root` is the STEP assembly whose scene instance is
        // captured; its Drivers pose the Root's `joints`; jobs arrive on
        // its WebSocket.
        [SerializeField] private StateServer m_StateServer;
        public StateServer StateServer => m_StateServer;

        // Owns the markers: applied to the instance at Start; its list
        // order is the dataset's class order.
        [SerializeField] private MarkerManager m_MarkerManager;
        public MarkerManager MarkerManager => m_MarkerManager;

        [Header("Camera")]
        [SerializeField] private Camera m_Camera;
        public Camera CaptureCamera
        {
            get
            {
                if (m_Camera == null)
                    m_Camera = Camera.main;

                return m_Camera;
            }
        }

        [Header("Lighting")]
        [SerializeField] private Light m_DirectionalLight;

        [Header("Robot Bounds")]
        [SerializeField] private RobotBounds m_RobotBounds;
        [SerializeField] private bool m_KeepBoundsInFrustrum = true;

        [Header("Catalog")]
        // Every desk, skybox and prop pose a generation set can name
        // (CatalogSO). Referencing it here is what builds those assets into
        // the player: Unity includes only what the build's scenes reference.
        [SerializeField] private CatalogSO m_Catalog;
        public CatalogSO Catalog => m_Catalog;

        // Set by Start: what every job uses.
        private Transform m_Instance;
        private List<JointDriver> m_Drivers;
        private Camera m_CaptureCamera;
        private Vector3 m_CameraBasePosition;
        private Quaternion m_CameraBaseRotation;

        // Why the generator cannot run jobs (null when it can): every job is
        // answered with this error rather than left unanswered.
        private string m_NotReadyReason;
        private bool m_IsRunning;

        // On start the generator listens for jobs on the StateServer, then
        // prepares what every job uses: the robot's instance
        // (StateServer.instance), the drivers, the markers (MarkerManager),
        // and the
        // camera's authored pose. A missing piece is logged and every job
        // is answered with it as the error.
        private void Start()
        {
            if (m_StateServer == null)
            {
                Debug.LogWarning($"{k_LogPrefix} assign the StateServer in the inspector - jobs arrive on it");
                return;
            }
            m_StateServer.BinaryReceived += StateServer_BinaryReceived;
            m_NotReadyReason = Prepare();
            if (m_NotReadyReason != null)
            {
                Debug.LogWarning($"{k_LogPrefix} not ready, every job will be refused: {m_NotReadyReason}");
                return;
            }
            Debug.Log($"{k_LogPrefix} waiting for generation jobs on {m_StateServer.Url}");
        }

        private void OnDestroy()
        {
            if (m_StateServer != null)
            {
                m_StateServer.BinaryReceived -= StateServer_BinaryReceived;
            }
        }

        // What every job uses, or why it is missing.
        private string Prepare()
        {
            if (m_MarkerManager == null)
            {
                return "no MarkerManager assigned in the inspector";
            }
            if (m_Catalog == null)
            {
                return "no CatalogSO assigned in the inspector";
            }
            var root = m_StateServer.root;
            if (root == null)
            {
                return $"StateServer '{m_StateServer.name}' has no Root assigned";
            }
            // The robot in the scene: the StateServer's scene reference, the
            // same instance its drivers were attached from.
            m_Instance = m_StateServer.instance;
            if (m_Instance == null)
            {
                return $"StateServer '{m_StateServer.name}' has no instance assigned (the robot in the scene)";
            }
            m_Drivers = DriversInJointOrder(root);
            if (m_Drivers.Count == 0)
            {
                return $"StateServer '{m_StateServer.name}' has no joint drivers - nothing to pose";
            }
            // Every marker placed, or none (MarkerManager logs why): a gap
            // would shift the classes.
            if (!m_MarkerManager.TryApply(m_Instance))
            {
                return "the MarkerManager did not place every marker (its log says why)";
            }
            m_CaptureCamera = CaptureCamera;
            if (m_CaptureCamera == null)
            {
                return "no capture camera - assign one in the inspector";
            }
            if (m_RobotBounds == null)
            {
                return "no RobotBounds assigned - the frustum fit needs it";
            }
            // The camera's authored pose, read once before any job runs:
            // every draw offsets it (lateral, height, pitch, yaw) and the
            // frustum fit then compensates along the view axis.
            m_CameraBasePosition = m_CaptureCamera.transform.position;
            m_CameraBaseRotation = m_CaptureCamera.transform.rotation;
            return null;
        }

        // The StateServer's drivers in the order of root.joints. The
        // runner draws each driver's value from the set's seeded random
        // sequence in list order, so the order must be fixed for a seed to
        // reproduce its states; Drivers is a dictionary, whose enumeration
        // order is not promised, so the Root's list sets it. A joint
        // StateServer skipped (not drivable, or a repeated name) has no
        // driver and is left out here too.
        private List<JointDriver> DriversInJointOrder(Root root)
        {
            var drivers = new List<JointDriver>();
            foreach (var joint in root.joints)
            {
                if (joint == null)
                {
                    continue;
                }
                foreach (var driver in m_StateServer.Drivers.Values)
                {
                    if (driver != null && driver.Source == joint)
                    {
                        drivers.Add(driver);
                        break;
                    }
                }
            }
            return drivers;
        }

        // One binary message on the StateServer (main thread): a job to
        // start, or an error reply saying why it cannot.
        private void StateServer_BinaryReceived(BinaryRequest request)
        {
            if (m_NotReadyReason != null)
            {
                ReplyError(request, $"the player is not ready: {m_NotReadyReason}");
                return;
            }
            if (m_IsRunning)
            {
                ReplyError(request, "a job is already running; send the next one after its GenerationDone");
                return;
            }
            Proto.GenerationJob job;
            try
            {
                job = Proto.GenerationJob.Parser.ParseFrom(request.Payload);
            }
            catch (InvalidProtocolBufferException exception)
            {
                ReplyError(request, $"the message is not a GenerationJob: {exception.Message}");
                return;
            }
            if (!ResolvedGenerationSet.TryResolve(job, m_Catalog, m_Drivers, out var set, out var error))
            {
                ReplyError(request, error);
                return;
            }
            RunJob(set, request);
        }

        // Runs one set and replies with its GenerationDone. The run is
        // async: each state is a synchronous main-thread chunk, and the
        // awaits between states hand the player loop back. While it runs
        // the StateServer's idle wait is 0: in a player, its Update would
        // otherwise block up to playerIdleWaitMilliseconds every frame
        // waiting for a message, and a run renders across many frames.
        private async void RunJob(ResolvedGenerationSet set, BinaryRequest request)
        {
            m_IsRunning = true;
            var idleWait = m_StateServer.playerIdleWaitMilliseconds;
            m_StateServer.playerIdleWaitMilliseconds = 0;
            var done = new Proto.GenerationDone();
            GenerationSetRunner runner = null;
            try
            {
                runner = new GenerationSetRunner(
                    set,
                    m_MarkerManager.Placed,
                    m_Drivers,
                    m_Instance.gameObject,
                    m_CaptureCamera,
                    m_DirectionalLight,
                    m_RobotBounds,
                    m_KeepBoundsInFrustrum,
                    m_CameraBasePosition,
                    m_CameraBaseRotation);
                await runner.RunAsync();
                done.Error = runner.Failure ?? string.Empty;
            }
            catch (Exception exception)
            {
                // An async void method: an exception not caught here would
                // reach the player loop and the client would never hear back.
                Debug.LogException(exception);
                done.Error = $"set '{set.Name}': the run threw {exception.GetType().Name}: {exception.Message}";
            }
            finally
            {
                m_StateServer.playerIdleWaitMilliseconds = idleWait;
                m_IsRunning = false;
            }
            if (runner != null)
            {
                done.RunDirectory = runner.RunDirectory ?? string.Empty;
                done.Frames = (uint)runner.Frames;
            }
            Debug.Log(
                $"{k_LogPrefix} '{set.Name}' done: {done.Frames} frames -> {done.RunDirectory}" +
                (done.Error.Length > 0 ? $" (error: {done.Error})" : string.Empty));
            request.Reply(done.ToByteArray());
        }

        private static void ReplyError(BinaryRequest request, string error)
        {
            Debug.LogWarning($"{k_LogPrefix} job refused: {error}");
            request.Reply(new Proto.GenerationDone { Error = error }.ToByteArray());
        }
    }
}
