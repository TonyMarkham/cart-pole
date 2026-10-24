using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Net.WebSockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using Greyhound.Step;
using Simulation.CartPole.SyntheticDataGen;

using UnityEngine;
using UnityEngine.SceneManagement;

namespace Simulation.CartPole.SimLoop
{
    // The simulated robot loop's Unity side, the Orin's camera
    // (synthetic-loop-dia.md, 6-7, 12-13): the orchestrator launches the
    // player with -orin ws://<host>:<port>, the synthetic Orin's frame
    // socket; this component connects there, and for every trigger - a
    // joint-state text message from the Orin - drives the cart and pole,
    // renders the camera, and answers with exactly one binary message, the
    // 728x544 greyscale q90 JPEG the Orin's detector would see. One reply
    // per trigger, in order: lockstep. Nothing is rendered between
    // triggers, and the window shows the last frame sent
    // (synthetic-loop-dia.md, Training instances, The players draw only
    // when asked).
    //
    // Message in: {"id": 7, "joints": [{"name": "02-Cart.Slider.joint",
    // "qpos": 0.05}, {"name": "04-Pole.Hinge.joint", "qpos": 3.14}]} - MJCF
    // joint names, MuJoCo's runtime units (slide meters, hinge radians).
    //
    // The joints are the StepJoint assets themselves: each holds its part's
    // sibling-index path and its MJCF mirror (type, axis, pivot), and its
    // asset name is the MJCF <joint name> - the wire key. The STEP instance
    // and its parts are resolved at runtime; nothing is authored on the
    // instance, and nothing resolved here is written back to it. The
    // rig's marker quads are added to the instance at startup (RigSO.Apply)
    // and exist only while the scene runs.
    public sealed class SimulationLoopController : MonoBehaviour
    {
        // Unity <-> MJCF swaps Y and Z (greyhound's exporter: Unity axis
        // (0,0,-1) is MJCF axis "0 -1 0"). A swap is a mirror, determinant
        // -1, and a mirror reverses rotation sense: +qpos about the MJCF
        // axis is -qpos about the Unity axis. Translations are unaffected,
        // so slides need no sign.
        private const int MjcfToUnityHingeSign = -1;

        // The frame contract (synthetic-loop-dia.md, Contracts): half the IMX296's native
        // 1456x1088. The 24 is the depth buffer's bit count - what lets
        // near surfaces hide far ones - matching the dataset generator.
        private const int FrameWidth = 728;
        private const int FrameHeight = 544;
        private const int DepthBits = 24;

        private const string OrinArgument = "-orin";

        // The player draws only when asked (synthetic-loop-dia.md,
        // Training instances): between triggers Update sleeps on the inbox
        // instead of spinning, and wakes at least this often so the
        // window stays responsive and a closed socket is noticed. A
        // trigger ends the wait as soon as it arrives. The editor never
        // waits: blocking its main thread would stall the editor itself.
        private const int IdleWaitMilliseconds = 100;

        [SerializeField] private StepJointSet m_JointSet;
        public StepJointSet JointSet => m_JointSet;

        [SerializeField] private StepJoint m_Hinge;
        public StepJoint Hinge => m_Hinge;

        [SerializeField] private StepJoint m_Slide;
        public StepJoint Slide => m_Slide;

        // The rig's markers are applied to the STEP instance at startup -
        // the same RigSO.Apply the dataset generator calls - so the
        // frames carry the markers the detector looks for. The quads are
        // created at runtime only.
        [SerializeField] private RigSO m_Rig;
        public RigSO Rig => m_Rig;

        [SerializeField] private Camera m_RenderCamera;
        public Camera RenderCamera => m_RenderCamera;

        // Editor Play mode has no command line: the orchestrator runs with
        // --no-launch, and this field is the synthetic Orin's frame socket
        // at the trainer's default unity_port. A new field rather than a
        // renamed one, so the old orchestrator address (port 9000, now the
        // control socket) is not carried over.
        [SerializeField] private string m_EditorOrinUrl = "ws://127.0.0.1:9001";
        public string EditorOrinUrl => m_EditorOrinUrl;

        [Serializable]
        private sealed class JointQpos
        {
            public string name = "";
            public float qpos = 0f;
        }

        [Serializable]
        private sealed class StateMessage
        {
            public int id = 0;
            public JointQpos[] joints = new JointQpos[0];
        }

        // A driven part's rest pose (qpos = 0), in its parent's space so a
        // child part follows its parent: the pole rides on the cart. Local
        // values are meters because greyhound bakes the import scale into
        // the vertices and leaves every transform at unit scale.
        private sealed class PartPose
        {
            public Transform node;
            public Vector3 localPosition;
            public Quaternion localRotation;
        }

        private PartPose m_HingePose;
        private PartPose m_SlidePose;
        private bool m_Ready;

        private RenderTexture m_Target;
        private Texture2D m_Readback;
        // The window shows the last frame sent, once there is one.
        private bool m_HasFrame;

        private ClientWebSocket m_Socket;
        private readonly CancellationTokenSource m_Cancel = new CancellationTokenSource();
        // Received on a background task, drained on the main thread: Unity
        // APIs are main-thread only. Blocking, so the main thread can sleep
        // until a trigger arrives.
        private readonly BlockingCollection<string> m_Inbox = new BlockingCollection<string>();
        // ClientWebSocket allows one send at a time; each reply chains on
        // the previous one, which also keeps replies in request order.
        private Task m_Sending = Task.CompletedTask;
        private volatile bool m_SocketEnded;
        private volatile bool m_SocketFailed;
        private readonly HashSet<string> m_UnknownJoints = new HashSet<string>();

        private void OnValidate()
        {
            foreach (var problem in ConfigurationProblems())
            {
                Debug.LogWarning($"[SimulationLoopController] '{name}': {problem}", this);
            }
        }

        // OnValidate runs only in the editor; a built player repeats the
        // same checks here and refuses to run on a broken configuration.
        private void Start()
        {
            var problems = ConfigurationProblems();
            if (problems.Count > 0)
            {
                foreach (var problem in problems)
                {
                    Debug.LogError($"[SimulationLoopController] {problem}", this);
                }
                Stop(1);
                return;
            }
            var instance = FindStepInstance();
            if (instance == null)
            {
                Debug.LogError(
                    $"[SimulationLoopController] no instance of '{m_JointSet.stepAssetPath}' " +
                    "in the active scene", this);
                Stop(1);
                return;
            }
            m_Rig.Apply(instance.gameObject);
            m_SlidePose = CapturePose(instance, m_Slide);
            m_HingePose = CapturePose(instance, m_Hinge);
            if (m_SlidePose == null || m_HingePose == null)
            {
                Stop(1);
                return;
            }

            m_Target = new RenderTexture(FrameWidth, FrameHeight, DepthBits);
            m_Readback = new Texture2D(FrameWidth, FrameHeight, TextureFormat.RGB24, false);
            // Disabled, Unity never renders the camera on its own; Render()
            // still renders it on demand, once per trigger.
            m_RenderCamera.enabled = false;

            var url = OrinUrl();
            if (url == null)
            {
                Debug.LogError(
                    $"[SimulationLoopController] no {OrinArgument} <url> on the " +
                    "command line - the orchestrator launches this player and passes it " +
                    "the synthetic Orin's frame socket",
                    this);
                Stop(1);
                return;
            }
            m_Ready = true;
            _ = Task.Run(() => ConnectAndReceive(url, m_Cancel.Token));
        }

        // Every rule a usable configuration must satisfy, as messages; an
        // empty list is a valid configuration.
        private List<string> ConfigurationProblems()
        {
            var problems = new List<string>();
            if (m_JointSet == null)
            {
                problems.Add("no StepJointSet assigned");
            }
            CheckJoint(problems, m_Hinge, "hinge", StepJointType.Hinge);
            CheckJoint(problems, m_Slide, "slide", StepJointType.Slide);
            if (m_Rig == null)
            {
                problems.Add("no RigSO assigned - the frames would carry no markers");
            }
            if (m_RenderCamera == null)
            {
                problems.Add("no render camera assigned");
            }
            return problems;
        }

        private void CheckJoint(List<string> problems, StepJoint joint, string role, StepJointType type)
        {
            if (joint == null)
            {
                problems.Add($"no {role} joint assigned");
                return;
            }
            if (joint.mj.type != type)
            {
                problems.Add($"{role} joint '{joint.name}' is a {joint.mj.type}, not a {type}");
            }
            if (m_JointSet != null && !m_JointSet.joints.Contains(joint))
            {
                problems.Add($"{role} joint '{joint.name}' is not in joint set '{m_JointSet.name}'");
            }
            // The exporter writes <joint name> as the asset name with every
            // character outside [A-Za-z0-9_.-] replaced by '_'; a name that
            // changes under that rule would never match what MuJoCo sends.
            foreach (var character in joint.name)
            {
                var allowed = (character >= 'a' && character <= 'z') ||
                    (character >= 'A' && character <= 'Z') ||
                    (character >= '0' && character <= '9') ||
                    character == '_' || character == '.' || character == '-';
                if (!allowed)
                {
                    problems.Add(
                        $"{role} joint name '{joint.name}' has '{character}', which the MJCF " +
                        "exporter rewrites to '_' - rename the asset to [A-Za-z0-9_.-] only");
                    break;
                }
            }
        }

        // The STEP instance, matched against the joint set's STEP asset
        // (SyntheticDataGenerator's rule): in the editor a scene root whose
        // prefab source is the stored STEP path, else - and in a player,
        // always - a root named after the STEP asset's file. Active scene
        // only.
        private Transform FindStepInstance()
        {
            var stepPath = m_JointSet.stepAssetPath;
            var stepName = Path.GetFileNameWithoutExtension(stepPath);
            foreach (var sceneRoot in SceneManager.GetActiveScene().GetRootGameObjects())
            {
#if UNITY_EDITOR
                if (!string.IsNullOrEmpty(stepPath))
                {
                    var source = UnityEditor.PrefabUtility.GetNearestPrefabInstanceRoot(sceneRoot);
                    if (source != null &&
                        UnityEditor.PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(source) == stepPath)
                    {
                        return sceneRoot.transform;
                    }
                }
#endif
                if (sceneRoot.name == stepName)
                {
                    return sceneRoot.transform;
                }
            }
            return null;
        }

        // The part a joint's sibling-index path walks to from the instance
        // root, with its rest pose; null (and an error) where the path no
        // longer resolves.
        private PartPose CapturePose(Transform root, StepJoint joint)
        {
            var node = root;
            foreach (var index in joint.body.indices)
            {
                if (node == null || index < 0 || index >= node.childCount)
                {
                    node = null;
                    break;
                }
                node = node.GetChild(index);
            }
            if (node == null || joint.body.indices.Length == 0)
            {
                Debug.LogError(
                    $"[SimulationLoopController] joint '{joint.name}': part '{joint.body.name}' " +
                    $"[{string.Join("/", joint.body.indices)}] not found under '{root.name}'",
                    this);
                return null;
            }
            return new PartPose
            {
                node = node,
                localPosition = node.localPosition,
                localRotation = node.localRotation,
            };
        }

        // Drives the parts to a state: each value goes to the joint whose
        // asset name equals its MJCF name. mj.axis and mj.pos are in the
        // part's own frame, so the motion is composed onto the rest pose in
        // that frame:
        //   slide: shift the rest position by axis * qpos meters;
        //   hinge: rotate by MjcfToUnityHingeSign * qpos about axis, then
        //          shift so the pivot (mj.pos) stays where it was at rest.
        public void Apply(string jointName, float qpos)
        {
            if (jointName == m_Slide.name)
            {
                m_SlidePose.node.localPosition = m_SlidePose.localPosition +
                    m_SlidePose.localRotation * (m_Slide.mj.axis.normalized * qpos);
            }
            else if (jointName == m_Hinge.name)
            {
                var degrees = MjcfToUnityHingeSign * qpos * Mathf.Rad2Deg;
                var swing = Quaternion.AngleAxis(degrees, m_Hinge.mj.axis.normalized);
                var pivot = m_Hinge.mj.pos;
                m_HingePose.node.localRotation = m_HingePose.localRotation * swing;
                m_HingePose.node.localPosition = m_HingePose.localPosition +
                    m_HingePose.localRotation * (pivot - swing * pivot);
            }
            else if (m_UnknownJoints.Add(jointName))
            {
                Debug.LogWarning(
                    $"[SimulationLoopController] ignoring unknown joint '{jointName}' " +
                    $"(driving '{m_Slide.name}' and '{m_Hinge.name}')", this);
            }
        }

        private void Update()
        {
            if (!m_Ready)
            {
                return;
            }
            var wait = Application.isEditor ? 0 : IdleWaitMilliseconds;
            if (m_Inbox.TryTake(out var json, wait))
            {
                Handle(json);
                while (m_Inbox.TryTake(out json))
                {
                    Handle(json);
                }
            }
            if (m_SocketEnded)
            {
                Debug.Log("[SimulationLoopController] synthetic Orin connection ended", this);
                // A launched player exists only for its run: the Orin
                // closing the frame socket ends it with 0 (24), a failed
                // connection with 1.
                Stop(m_SocketFailed ? 1 : 0);
            }
        }

        private void Handle(string json)
        {
            StateMessage message;
            try
            {
                message = JsonUtility.FromJson<StateMessage>(json);
            }
            catch (Exception exception)
            {
                Debug.LogWarning(
                    $"[SimulationLoopController] unparsable state message: {exception.Message}", this);
                return;
            }
            if (message?.joints != null)
            {
                foreach (var joint in message.joints)
                {
                    Apply(joint.name, joint.qpos);
                }
            }
            var jpeg = Render();
            var socket = m_Socket;
            var token = m_Cancel.Token;
            m_Sending = m_Sending.ContinueWith(
                _ => socket.SendAsync(
                    new ArraySegment<byte>(jpeg), WebSocketMessageType.Binary, true, token),
                token, TaskContinuationOptions.None, TaskScheduler.Default).Unwrap();
        }

        // One frame: the camera rendered into the frame-sized target,
        // read back, and encoded by the dataset generator's own greyscale
        // encoder - the detector's training domain.
        private byte[] Render()
        {
            var previousActive = RenderTexture.active;
            var previousTarget = m_RenderCamera.targetTexture;
            m_RenderCamera.targetTexture = m_Target;
            m_RenderCamera.Render();
            m_RenderCamera.targetTexture = previousTarget;
            RenderTexture.active = m_Target;
            m_Readback.ReadPixels(new Rect(0, 0, FrameWidth, FrameHeight), 0, 0);
            m_Readback.Apply();
            RenderTexture.active = previousActive;
            m_HasFrame = true;
            return GreyscaleEncoder.Encode(m_Readback);
        }

        // The window shows the last frame rendered, stretched to fill it
        // (960x720 against 728x544 is 0.4% off in aspect): with the camera
        // disabled, nothing else draws there.
        private void OnGUI()
        {
            if (!m_HasFrame || Event.current.type != EventType.Repaint)
            {
                return;
            }
            GUI.DrawTexture(new Rect(0, 0, Screen.width, Screen.height), m_Target, ScaleMode.StretchToFill, false);
        }

        // In a player the address comes only from the command line - the
        // orchestrator that launched it; in the editor, from the inspector.
        private string OrinUrl()
        {
            if (Application.isEditor)
            {
                return m_EditorOrinUrl;
            }
            var args = Environment.GetCommandLineArgs();
            for (var i = 0; i < args.Length - 1; i++)
            {
                if (args[i] == OrinArgument)
                {
                    return args[i + 1];
                }
            }
            return null;
        }

        // Background task: connect, then queue every text message until the
        // synthetic Orin closes the socket or the component goes away.
        private async Task ConnectAndReceive(string url, CancellationToken token)
        {
            try
            {
                m_Socket = new ClientWebSocket();
                await m_Socket.ConnectAsync(new Uri(url), token);
                Debug.Log($"[SimulationLoopController] connected to {url}");
                var buffer = new byte[4096];
                var message = new MemoryStream();
                while (m_Socket.State == WebSocketState.Open)
                {
                    var result = await m_Socket.ReceiveAsync(new ArraySegment<byte>(buffer), token);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        await m_Socket.CloseOutputAsync(
                            WebSocketCloseStatus.NormalClosure, "closed by the synthetic Orin", token);
                        break;
                    }
                    message.Write(buffer, 0, result.Count);
                    if (!result.EndOfMessage)
                    {
                        continue;
                    }
                    if (result.MessageType == WebSocketMessageType.Text)
                    {
                        m_Inbox.Add(Encoding.UTF8.GetString(message.GetBuffer(), 0, (int)message.Length));
                    }
                    message.SetLength(0);
                }
            }
            catch (OperationCanceledException)
            {
                // The component is shutting down.
            }
            catch (Exception exception)
            {
                m_SocketFailed = true;
                Debug.LogError($"[SimulationLoopController] connection to {url} failed: {exception.Message}");
            }
            finally
            {
                m_SocketEnded = true;
            }
        }

        // Ends a player with the given exit code (non-zero for a
        // configuration or connection failure, so the orchestrator sees
        // it); editor Play mode stays up so the scene can be inspected,
        // and only the driving stops.
        private void Stop(int exitCode)
        {
            m_Ready = false;
            if (!Application.isEditor)
            {
                Application.Quit(exitCode);
            }
        }

        private void OnDestroy()
        {
            m_Cancel.Cancel();
            m_Socket?.Dispose();
            if (m_Target != null)
            {
                m_Target.Release();
                Destroy(m_Target);
            }
            if (m_Readback != null)
            {
                Destroy(m_Readback);
            }
        }
    }
}
