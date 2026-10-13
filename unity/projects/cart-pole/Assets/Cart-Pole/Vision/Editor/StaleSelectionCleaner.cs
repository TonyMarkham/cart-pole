using UnityEditor;
using UnityEditor.SceneManagement;

using UnityEngine;

namespace CartPole.Vision.Editor
{
    // Clears editor selections pointing at destroyed objects. A corpse
    // selection survives domain reloads and makes the inspector throw
    // MissingReferenceException on every reload - GameObjectInspector
    // re-enabling against a destroyed target (e.g. a play-session desk
    // that was selected when play mode exited). Unity's overloaded ==
    // treats destroyed references as null, which is the corpse test.
    [InitializeOnLoad]
    internal static class StaleSelectionCleaner
    {
        static StaleSelectionCleaner()
        {
            EditorApplication.delayCall += Clear;
        }

        private static void Clear()
        {
            var selected = Selection.objects;
            var alive = System.Array.FindAll(selected, o => o != null);
            if (alive.Length != selected.Length)
            {
                Selection.objects = alive;
                Debug.Log(
                    $"[StaleSelectionCleaner] cleared {selected.Length - alive.Length} " +
                    "destroyed object(s) from the selection");
            }
        }

        // Forces a script reload on the next editor tick - the domain
        // reload that would otherwise require a file change or editor
        // focus.
        [MenuItem("Cart-Pole/Force Domain Reload")]
        private static void ForceReload()
        {
            EditorUtility.RequestScriptReload();
        }

        // Closes the editor and re-opens the same project, via
        // OpenProject (Unity quits and relaunches). Open scenes and
        // assets are saved first so nothing unsaved is lost.
        [MenuItem("Cart-Pole/Restart Unity Project")]
        private static void Restart()
        {
            EditorSceneManager.SaveOpenScenes();
            AssetDatabase.SaveAssets();
            EditorApplication.OpenProject(
                System.IO.Directory.GetParent(Application.dataPath).FullName);
        }
    }
}
