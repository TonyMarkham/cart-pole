using System.Collections.Generic;

using Greyhound.MuJoCo;

using Simulation.CartPole.SyntheticDataGen;

using UnityEngine;

namespace Simulation.CartPole.SimLoop
{
    // The simulated robot loop's camera (__documentation__/synthetic-loop-dia.md,
    // 12-13): for every joint state the StateServer on this GameObject
    // applies, it renders the camera and replies to the client that sent
    // the state with exactly one binary message, the greyscale JPEG the
    // Orin's detector would see. One reply per state, in order: lockstep.
    // The camera also draws to the window every frame as usual; the reply
    // is a separate render into the frame-sized target, made right after
    // the state's joints are posed, so the window never affects a reply.
    //
    // It lives on one GameObject with the StateServer (receives the states
    // and poses the joints, from its Joint assets) and the MarkerManager
    // (places the robot's ArUco markers at Start, as children of the parts,
    // so they move with them). This component only renders and replies.
    //
    // Frame: m_FrameWidth x m_FrameHeight pixels with a m_DepthBits depth
    // buffer, encoded by the dataset generator's GreyscaleEncoder at its
    // fixed quality (GreyscaleEncoder.Quality, 90): the detector's training
    // domain. The defaults are the frame contract
    // (__documentation__/synthetic-loop-dia.md, Contracts): 728x544, half
    // the IMX296's native 1456x1088, and 24 depth bits (what lets near
    // surfaces hide far ones), matching the dataset generator.
    //
    // Environment: the scene the camera sees is set once, at Start - the
    // desk (m_Desk, placed by DeskSO.Spawn exactly as the generator places
    // it), the skybox HDRI (m_Skybox) turned m_SkyboxYawDegrees about the
    // world's up axis, and the RobotBounds cube's GameObject turned off so
    // the frustum-fit helper never shows in a frame. Each is optional: one
    // left empty leaves the scene as it is. Referencing the desk and HDRI
    // here also builds them into the player.
    //
    // Lifetime: a built player exists only for its run. When its last
    // client disconnects, after at least one connected, it quits with 0
    // (the Orin closing its socket ends the run, 24); a configuration
    // problem or a StateServer that is not listening quits it with 1.
    // Editor Play mode never quits, so the scene can be inspected.
    [RequireComponent(typeof(StateServer))]
    public sealed class SimulationLoopController : MonoBehaviour
    {
        [SerializeField] private Camera m_RenderCamera;
        public Camera RenderCamera => m_RenderCamera;

        [SerializeField] private int m_FrameWidth = 728;
        [SerializeField] private int m_FrameHeight = 544;

        // Depth buffer bits; Unity accepts 0, 16, 24 or 32.
        [SerializeField] private int m_DepthBits = 24;

        [Header("Environment")]
        [SerializeField] private DeskSO m_Desk;
        [SerializeField] private Material m_Skybox;

        // Degrees about the world's up axis (Y), written to the skybox
        // material's rotation: 0 is the HDRI as authored, 90 turns it a
        // quarter turn.
        [SerializeField] private float m_SkyboxYawDegrees;

        // The frustum-fit helper cube; its GameObject is turned off.
        [SerializeField] private RobotBounds m_RobotBounds;

        // The skybox shaders' rotation property (Skybox/Panoramic and
        // Skybox/Cubemap): degrees about the world's up axis.
        private const string k_SkyboxRotationProperty = "_Rotation";

        private StateServer m_Server;
        private bool m_Ready;
        private bool m_HadClient;

        private RenderTexture m_Target;
        private Texture2D m_Readback;

        private void OnValidate()
        {
            foreach (var problem in ConfigurationProblems())
            {
                Debug.LogWarning($"[SimulationLoopController] '{name}': {problem}", this);
            }
        }

        private void Awake()
        {
            m_Server = GetComponent<StateServer>();
        }

        // Subscribed before any Update, so no applied state is missed; the
        // handler ignores states until Start has made the frame target.
        // RequireComponent only adds the StateServer when this component is
        // added; a GameObject that had this component before it did may
        // lack one, which Start reports.
        private void OnEnable()
        {
            if (m_Server != null)
            {
                m_Server.StateApplied += OnStateApplied;
            }
        }

        private void OnDisable()
        {
            if (m_Server != null)
            {
                m_Server.StateApplied -= OnStateApplied;
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
            if (!m_Server.Listening)
            {
                Debug.LogError("[SimulationLoopController] the StateServer is not listening (see its error above)", this);
                Stop(1);
                return;
            }

            ApplyEnvironment();
            m_Target = new RenderTexture(m_FrameWidth, m_FrameHeight, m_DepthBits);
            m_Readback = new Texture2D(m_FrameWidth, m_FrameHeight, TextureFormat.RGB24, false);
            m_Ready = true;
        }

        // The desk, the turned skybox and the hidden bounds cube (see
        // Environment above). The skybox is turned on a copy of m_Skybox,
        // so the material asset is never changed; the ambient light and
        // reflections are then recomputed from the turned sky
        // (DynamicGI.UpdateEnvironment), so the lighting matches what the
        // camera sees.
        private void ApplyEnvironment()
        {
            if (m_Desk != null)
            {
                m_Desk.Spawn();
            }
            if (m_Skybox != null)
            {
                var skybox = new Material(m_Skybox);
                skybox.SetFloat(k_SkyboxRotationProperty, m_SkyboxYawDegrees);
                RenderSettings.skybox = skybox;
                DynamicGI.UpdateEnvironment();
            }
            if (m_RobotBounds != null)
            {
                m_RobotBounds.gameObject.SetActive(false);
            }
        }

        // Every rule a usable configuration must satisfy, as messages; an
        // empty list is a valid configuration.
        private List<string> ConfigurationProblems()
        {
            var problems = new List<string>();
            if (GetComponent<StateServer>() == null)
            {
                problems.Add("no StateServer on this GameObject - add one; it receives the states this camera answers");
            }
            if (m_RenderCamera == null)
            {
                problems.Add("no render camera assigned");
            }
            if (m_FrameWidth <= 0 || m_FrameHeight <= 0)
            {
                problems.Add($"frame size {m_FrameWidth}x{m_FrameHeight} must be positive");
            }
            if (m_DepthBits != 0 && m_DepthBits != 16 && m_DepthBits != 24 && m_DepthBits != 32)
            {
                problems.Add($"depth bits {m_DepthBits} must be 0, 16, 24 or 32");
            }
            return problems;
        }

        private void OnStateApplied(StateRequest request)
        {
            if (!m_Ready)
            {
                return;
            }
            request.Reply(Render());
        }

        private void Update()
        {
            if (!m_Ready)
            {
                return;
            }
            if (m_Server == null)
            {
                return;
            }
            if (m_Server.Clients > 0)
            {
                m_HadClient = true;
            }
            else if (m_HadClient)
            {
                Debug.Log("[SimulationLoopController] the last client disconnected", this);
                // A launched player exists only for its run.
                Stop(0);
            }
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
            m_Readback.ReadPixels(new Rect(0, 0, m_FrameWidth, m_FrameHeight), 0, 0);
            m_Readback.Apply();
            RenderTexture.active = previousActive;
            return GreyscaleEncoder.Encode(m_Readback);
        }

        // Ends a player with the given exit code (non-zero for a
        // configuration or listening failure, so the orchestrator sees it);
        // editor Play mode stays up so the scene can be inspected, and only
        // the rendering stops.
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
