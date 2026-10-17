using System;
using System.IO;
using System.Linq;

using UnityEditor;
using UnityEditor.UIElements;

using Simulation.CartPole.SyntheticDataGen;
using UnityEngine;
using UnityEngine.UIElements;

namespace CartPole.Vision.Editor
{
    // Inspector for ForcedSituationSO: dropping a frame's
    // frame_NNNNNN_state.json sidecar in populates every field from
    // it - the state, the camera pose, and the desk/skybox/light
    // references resolved from the recorded asset paths. The sidecars
    // are filesystem files in the dataset's wip~ folder (never
    // imported by the AssetDatabase, so there is no Unity asset to
    // drag from the Project view): the drop zone accepts OS file
    // drags, and a Load button opens the file dialog. Save writes the
    // asset, like the other SO editors. Pure UIElements.
    [CustomEditor(typeof(ForcedSituationSO))]
    internal sealed class ForcedSituationEditor : UnityEditor.Editor
    {
        public override VisualElement CreateInspectorGUI()
        {
            var root = new VisualElement();
            foreach (var property in new[]
                     {
                         "label", "cartPosition", "poleRotation",
                         "useCameraPosition", "cameraPosition", "cameraRotation",
                         "useDesk", "desk", "useSkybox", "skybox", "skyboxYaw",
                         "useLight", "lightIntensity", "lightYaw",
                     })
            {
                root.Add(new PropertyField(serializedObject.FindProperty(property)));
            }

            var drop = new VisualElement();
            drop.style.height = 44f;
            drop.style.marginTop = 8f;
            drop.style.borderLeftWidth = 1f;
            drop.style.borderRightWidth = 1f;
            drop.style.borderTopWidth = 1f;
            drop.style.borderBottomWidth = 1f;
            drop.style.borderTopLeftRadius = 3f;
            drop.style.borderTopRightRadius = 3f;
            drop.style.borderBottomLeftRadius = 3f;
            drop.style.borderBottomRightRadius = 3f;
            drop.style.borderLeftColor = new Color(0.35f, 0.35f, 0.35f);
            drop.style.borderRightColor = new Color(0.35f, 0.35f, 0.35f);
            drop.style.borderTopColor = new Color(0.35f, 0.35f, 0.35f);
            drop.style.borderBottomColor = new Color(0.35f, 0.35f, 0.35f);
            drop.style.justifyContent = Justify.Center;
            drop.style.alignItems = Align.Center;
            drop.Add(new Label("drop a frame_NNNNNN_state.json from the dataset's wip~ folder"));
            drop.Add(new Label("to populate every field - or pick it from disk:"));
            drop.RegisterCallback<DragUpdatedEvent>(e =>
            {
                DragAndDrop.visualMode = DroppedJsonPath() != null
                    ? DragAndDropVisualMode.Copy
                    : DragAndDropVisualMode.Rejected;
                e.StopPropagation();
            });
            drop.RegisterCallback<DragPerformEvent>(e =>
            {
                var path = DroppedJsonPath();
                if (path != null)
                {
                    Populate(path);
                    DragAndDrop.AcceptDrag();
                }
                e.StopPropagation();
            });
            root.Add(drop);

            // The state jsons are filesystem files under the dataset's
            // wip~ sidecar folder - never imported by the AssetDatabase,
            // so there is no Unity asset to drag from the Project view.
            // The file dialog is the deterministic way in; the drop zone
            // keeps accepting OS drags (DragAndDrop.paths carries the
            // real file paths for files dragged from the OS explorer).
            var load = new Button(LoadFromDialog) { text = "Load state json from disk..." };
            load.style.marginTop = 4f;
            root.Add(load);

            var save = new Button(Save) { text = "Save" };
            save.style.marginTop = 8f;
            root.Add(save);
            return root;
        }

        private static string DroppedJsonPath()
        {
            // The state jsons are filesystem files under the dataset's
            // wip~ folder - never imported, invisible to the Project
            // view - so a drag that reaches here comes from the OS file
            // explorer, and DragAndDrop.paths carries the real
            // filesystem path.
            return DragAndDrop.paths?.FirstOrDefault(p =>
                p.EndsWith(".json", StringComparison.OrdinalIgnoreCase));
        }

        // The dialog starts in the dataset's wip~ sidecar folder when
        // it exists (the capture component's default output), else the
        // Assets root; afterwards it starts where the last load came
        // from (session-scoped - nothing persisted to preferences).
        private const string LastJsonDirectoryKey =
            "ForcedSituationEditor.LastJsonDirectory";

        private void LoadFromDialog()
        {
            var directory = SessionState.GetString(
                LastJsonDirectoryKey, DefaultJsonDirectory());
            var path = EditorUtility.OpenFilePanel(
                "Load frame state json", directory, "json");
            if (string.IsNullOrEmpty(path))
            {
                return;
            }
            SessionState.SetString(
                LastJsonDirectoryKey, Path.GetDirectoryName(path) ?? directory);
            Populate(path);
        }

        private static string DefaultJsonDirectory()
        {
            var wip = Path.Combine(Application.dataPath, "Captures/cart-pole/wip~");
            return Directory.Exists(wip) ? wip : Application.dataPath;
        }

        // The frame sidecar's field set mirrors the manifest entry; every
        // SO field is populated from it, including the desk and skybox
        // references resolved from the recorded asset paths.
        [Serializable]
        private sealed class FrameState
        {
            public string frame;
            public string situation;
            public float cartPosition;
            public float poleRotation;
            public bool useDesk;
            public string deskPath;
            public bool useSkybox;
            public string skyboxPath;
            public float skyboxYaw;
            public bool useLight;
            public float lightIntensity;
            public float lightYaw;
            public bool useCameraPosition;
            public Vector3 cameraPosition;
            public Vector3 cameraRotation;
        }

        private void Populate(string jsonPath)
        {
            var frame = JsonUtility.FromJson<FrameState>(File.ReadAllText(jsonPath));
            if (frame == null)
            {
                Debug.LogWarning($"[ForcedSituation] '{jsonPath}' is not a frame state file");
                return;
            }
            serializedObject.Update();
            serializedObject.FindProperty("label").stringValue =
                !string.IsNullOrEmpty(frame.situation)
                    ? frame.situation
                    : Path.GetFileNameWithoutExtension(jsonPath);
            serializedObject.FindProperty("cartPosition").floatValue = frame.cartPosition;
            serializedObject.FindProperty("poleRotation").floatValue = frame.poleRotation;
            serializedObject.FindProperty("useCameraPosition").boolValue =
                frame.useCameraPosition;
            serializedObject.FindProperty("cameraPosition").vector3Value =
                frame.cameraPosition;
            serializedObject.FindProperty("cameraRotation").vector3Value =
                frame.cameraRotation;

            var desk = string.IsNullOrEmpty(frame.deskPath)
                ? null
                : AssetDatabase.LoadAssetAtPath<DeskSO>(frame.deskPath);
            if (frame.useDesk && desk == null)
            {
                Debug.LogWarning(
                    $"[ForcedSituation] no DeskSO at '{frame.deskPath}' - desk left unassigned");
            }
            serializedObject.FindProperty("useDesk").boolValue = frame.useDesk;
            serializedObject.FindProperty("desk").objectReferenceValue = desk;

            var skybox = string.IsNullOrEmpty(frame.skyboxPath)
                ? null
                : AssetDatabase.LoadAssetAtPath<Material>(frame.skyboxPath);
            if (frame.useSkybox && skybox == null)
            {
                Debug.LogWarning(
                    $"[ForcedSituation] no skybox material at '{frame.skyboxPath}' - skybox left unassigned");
            }
            serializedObject.FindProperty("useSkybox").boolValue = frame.useSkybox;
            serializedObject.FindProperty("skybox").objectReferenceValue = skybox;
            serializedObject.FindProperty("skyboxYaw").floatValue = frame.skyboxYaw;

            serializedObject.FindProperty("useLight").boolValue = frame.useLight;
            serializedObject.FindProperty("lightIntensity").floatValue =
                frame.lightIntensity;
            serializedObject.FindProperty("lightYaw").floatValue = frame.lightYaw;
            serializedObject.ApplyModifiedProperties();
        }

        private void Save()
        {
            EditorUtility.SetDirty(target);
            AssetDatabase.SaveAssetIfDirty(target);
        }
    }
}
