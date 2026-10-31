using System;
using System.Globalization;

using UnityEngine;

namespace Simulation.CartPole.SimLoop
{
    // The standalone player's own window (synthetic-loop-dia.md, Training
    // instances, The windows): the orchestrator lays the training
    // instances' windows out in a grid and launches each player with
    // -window-position x,y, pixels from the top-left corner of the display
    // the window opens on; this component moves the window there once, at
    // startup. Without the argument, and in the editor, it does nothing.
    // A malformed value is logged and the window stays where it opened:
    // its place is for watching, never for the frames, which render into
    // their own 728x544 target whatever the window's size.
    public sealed class PlayerController : MonoBehaviour
    {
        private const string WindowPositionArgument = "-window-position";

        private void Start()
        {
            if (Application.isEditor)
            {
                return;
            }
            var text = ArgumentValue(WindowPositionArgument);
            if (text == null)
            {
                return;
            }
            if (!TryParsePosition(text, out var position))
            {
                Debug.LogError(
                    $"[PlayerController] {WindowPositionArgument} {text}: expected x,y in whole " +
                    "pixels - the window stays where it opened", this);
                return;
            }
            var display = Screen.mainWindowDisplayInfo;
            Screen.MoveMainWindowTo(display, position);
            Debug.Log($"[PlayerController] window moved to {position.x},{position.y} on '{display.name}'", this);
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
            var parts = text.Split(',');
            if (parts.Length != 2 ||
                !int.TryParse(parts[0].Trim(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var x) ||
                !int.TryParse(parts[1].Trim(), NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out var y))
            {
                return false;
            }
            position = new Vector2Int(x, y);
            return true;
        }
    }
}
