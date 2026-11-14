using System;
using System.Globalization;

using Simulation.CartPole.SyntheticDataGen;

using UnityEngine;

namespace Simulation.CartPole.SimLoop
{
    // The player's identity and its window.
    //
    // Mode: one scene serves both PlayerModes. On the same GameObject sit
    // SimulationLoopController (SyntheticCamera) and SyntheticDataGenerator
    // (SyntheticGenerator); at Awake this component turns on the chosen
    // mode's component and turns off the other, so only one of them ever
    // starts (a disabled component's Start never runs). The mode is
    // m_Mode (Inspector) unless the command line overrides it:
    //   -player-mode SyntheticCamera | SyntheticGenerator  (case ignored)
    // A malformed value, or no component for the chosen mode, is an error:
    // both components are turned off, so neither starts, and a player quits
    // with 1 rather than run as the wrong thing.
    //
    // It runs before the scene's other scripts (DefaultExecutionOrder below
    // 0), so the switch happens before either component's Awake / OnEnable.
    //
    // Window (__documentation__/synthetic-loop-dia.md, Training instances,
    // The windows): the orchestrator lays the instances' windows out in a
    // grid and launches each player with -window-position x,y, pixels from
    // the top-left corner of the display the window opens on; this component
    // moves the window there once, at Start. Without the argument, and in the
    // editor, it does nothing. A malformed value is logged and the window
    // stays where it opened: its place is for watching, never for the frames,
    // which render into their own target whatever the window's size.
    //
    // Camera: -camera-transform px,py,pz,rx,ry,rz places m_Camera at world
    // position (px, py, pz) in meters with Euler rotation (rx, ry, rz) in
    // degrees, at Awake, before any other script reads the camera's pose
    // (the generator takes it as its base pose at Start). Example:
    //   -camera-transform 0,1.1,-2.5,0,0,0
    // Without the argument the camera keeps its pose in the scene. A
    // malformed value, or no m_Camera, fails like a bad -player-mode: a
    // camera in the wrong place would make wrong frames, not just a
    // misplaced window.
    [DefaultExecutionOrder(-100)]
    public sealed class PlayerController : MonoBehaviour
    {
        // The command-line argument that overrides m_Mode, followed by a
        // PlayerMode name: -player-mode SyntheticGenerator.
        private const string k_PlayerModeArgument = "-player-mode";

        // The command-line argument that places the window, followed by
        // x,y in whole pixels: -window-position 0,0.
        private const string k_WindowPositionArgument = "-window-position";

        // -window-position's value is two integers split by this: "x,y".
        private const char k_PositionSeparator = ',';
        private const int k_PositionParts = 2;

        // The command-line argument that places m_Camera, followed by
        // px,py,pz,rx,ry,rz: -camera-transform 0,1.1,-2.5,0,0,0.
        private const string k_CameraTransformArgument = "-camera-transform";

        // -camera-transform's value: position x, y, z then rotation x, y, z.
        private const int k_CameraTransformParts = 6;

        // Every log line starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[PlayerController]";

        [SerializeField] private PlayerMode m_Mode = PlayerMode.SyntheticCamera;

        // The camera -camera-transform places.
        [SerializeField] private Camera m_Camera;

        // The mode this player runs as: m_Mode, or -player-mode's value once
        // Awake has read it.
        public PlayerMode Mode => m_Mode;

        private void Awake()
        {
            var loopController = GetComponent<SimulationLoopController>();
            var generator = GetComponent<SyntheticDataGenerator>();

            var text = ArgumentValue(k_PlayerModeArgument);
            if (text != null)
            {
                if (!Enum.TryParse(text, true, out PlayerMode mode) || !Enum.IsDefined(typeof(PlayerMode), mode))
                {
                    Fail(loopController, generator,
                        $"{k_PlayerModeArgument} {text}: expected {string.Join(" or ", Enum.GetNames(typeof(PlayerMode)))}");
                    return;
                }
                m_Mode = mode;
            }

            var hasModeComponent = m_Mode == PlayerMode.SyntheticCamera ? loopController != null : generator != null;
            if (!hasModeComponent)
            {
                Fail(loopController, generator,
                    $"mode {m_Mode} needs a " +
                    $"{(m_Mode == PlayerMode.SyntheticCamera ? nameof(SimulationLoopController) : nameof(SyntheticDataGenerator))} " +
                    "on this GameObject");
                return;
            }

            if (loopController != null)
            {
                loopController.enabled = m_Mode == PlayerMode.SyntheticCamera;
            }
            if (generator != null)
            {
                generator.enabled = m_Mode == PlayerMode.SyntheticGenerator;
            }
            Debug.Log($"{k_LogPrefix} running as {m_Mode}", this);

            var cameraText = ArgumentValue(k_CameraTransformArgument);
            if (cameraText == null)
            {
                return;
            }
            if (!TryParseCameraTransform(cameraText, out var position, out var eulerRotation))
            {
                Fail(loopController, generator,
                    $"{k_CameraTransformArgument} {cameraText}: expected px,py,pz,rx,ry,rz " +
                    "(meters, then degrees)");
                return;
            }
            if (m_Camera == null)
            {
                Fail(loopController, generator,
                    $"{k_CameraTransformArgument} given but no Camera is assigned");
                return;
            }
            m_Camera.transform.SetPositionAndRotation(position, Quaternion.Euler(eulerRotation));
            Debug.Log(
                $"{k_LogPrefix} camera placed at {position} with rotation {eulerRotation}", this);
        }

        private void Start()
        {
            if (Application.isEditor)
            {
                return;
            }
            var text = ArgumentValue(k_WindowPositionArgument);
            if (text == null)
            {
                return;
            }
            if (!TryParsePosition(text, out var position))
            {
                Debug.LogError(
                    $"{k_LogPrefix} {k_WindowPositionArgument} {text}: expected x,y in whole " +
                    "pixels - the window stays where it opened", this);
                return;
            }
            var display = Screen.mainWindowDisplayInfo;
            Screen.MoveMainWindowTo(display, position);
            Debug.Log($"{k_LogPrefix} window moved to {position.x},{position.y} on '{display.name}'", this);
        }

        // Turns both modes' components off, so neither starts (a player's
        // Quit takes effect only at the end of the frame), logs the problem,
        // and quits a player with 1; editor Play mode stays up so the scene
        // can be inspected.
        private void Fail(SimulationLoopController loopController, SyntheticDataGenerator generator, string problem)
        {
            if (loopController != null)
            {
                loopController.enabled = false;
            }
            if (generator != null)
            {
                generator.enabled = false;
            }
            Debug.LogError($"{k_LogPrefix} {problem}", this);
            if (!Application.isEditor)
            {
                Application.Quit(1);
            }
        }

        // The value after the named argument on the command line, else null.
        private static string ArgumentValue(string name)
        {
            var args = Environment.GetCommandLineArgs();
            for (var i = 0; i < args.Length - 1; i++)
            {
                if (args[i] == name)
                {
                    return args[i + 1];
                }
            }
            return null;
        }

        private static bool TryParsePosition(string text, out Vector2Int position)
        {
            position = default;
            var parts = text.Split(k_PositionSeparator);
            if (parts.Length != k_PositionParts ||
                !int.TryParse(parts[0].Trim(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var x) ||
                !int.TryParse(parts[1].Trim(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var y))
            {
                return false;
            }
            position = new Vector2Int(x, y);
            return true;
        }

        private static bool TryParseCameraTransform(string text, out Vector3 position, out Vector3 eulerRotation)
        {
            position = default;
            eulerRotation = default;
            var parts = text.Split(k_PositionSeparator);
            if (parts.Length != k_CameraTransformParts)
            {
                return false;
            }
            var values = new float[k_CameraTransformParts];
            for (var i = 0; i < k_CameraTransformParts; i++)
            {
                if (!float.TryParse(parts[i].Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out values[i]) ||
                    !float.IsFinite(values[i]))
                {
                    return false;
                }
            }
            position = new Vector3(values[0], values[1], values[2]);
            eulerRotation = new Vector3(values[3], values[4], values[5]);
            return true;
        }
    }
}
