using System;
using System.IO;
using System.Linq;

using UnityEditor;
using UnityEditor.UIElements;

using UnityEngine;
using UnityEngine.UIElements;

namespace CartPole.Vision.Editor
{
    // a frame's frame_NNNNNN_state.json sidecar in and every field populates
    // from it: the state, the camera pose, and the desk/skybox/light
    // references resolved from the recorded asset paths. Save writes the
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
            drop.Add(new Label("drop a frame_NNNNNN_state.json to populate every field"));
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

            var save = new Button(Save) { text = "Save" };
            save.style.marginTop = 8f;
            root.Add(save);
            return root;
        }

        private static string DroppedJsonPath()
        {
            // The file has the real .json extension on disk; the Project
            // view only hides it in the display name.
            return DragAndDrop.paths?.FirstOrDefault(p =>
                p.EndsWith(".json", StringComparison.OrdinalIgnoreCase));
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
