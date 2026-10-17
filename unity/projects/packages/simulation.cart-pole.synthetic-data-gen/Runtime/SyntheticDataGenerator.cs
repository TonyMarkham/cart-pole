using System.Collections.Generic;
using Unity.Mathematics;
using UnityEngine;
using UnityEngine.SceneManagement;

// ReSharper disable InconsistentNaming
// ReSharper disable UnusedMember.Global

namespace Simulation.CartPole.SyntheticDataGen
{
    // The dataset generator: one component owning the synth capture that
    // the separate RigStateRandomizer + RigStateCapture pair used to
    // split. It drives the rig through states and renders the labeled
    // frames - the rig is what it drives.
    public sealed class SyntheticDataGenerator : MonoBehaviour
    {
        [Header("Rig")]
        [SerializeField] private RigSO m_Rig;
        public RigSO Rig => m_Rig;

        [Header("Camera")]
        [SerializeField] private Camera m_Camera;
        public Camera camera
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

        [Header("Sets")]
        [SerializeField] private List<GenerationSetSO> m_Sets;

        // On start the generator drives the marker application: it asks
        // the rig for its STEP asset details, finds the assembly's
        // instance in the active scene itself, and hands the instance to
        // the rig - the rig never searches scene elements. With the
        // markers applied it runs every assigned set synchronously -
        // play mode only (editor-mode generation is rejected: it makes
        // crap data), one set after another, each writing its own
        // run_<timestamp> folder under its output directory.
        private void Start()
        {
            if (!Application.isPlaying)
            {
                return;
            }
            if (m_Rig == null)
            {
                Debug.LogWarning("[SyntheticDataGenerator] no rig assigned - assign the Rig SO in the inspector");
                return;
            }
            var instance = FindStepAssetInstance();
            if (instance == null)
            {
                return;
            }
            m_Rig.Apply(instance.gameObject);
            var camera = this.camera;
            if (camera == null)
            {
                Debug.LogWarning("[SyntheticDataGenerator] no capture camera - assign one in the inspector");
                return;
            }
            if (m_RobotBounds == null)
            {
                Debug.LogWarning("[SyntheticDataGenerator] no RobotBounds assigned - the frustum fit needs it");
                return;
            }
            if (m_Sets == null || m_Sets.Count == 0)
            {
                Debug.LogWarning("[SyntheticDataGenerator] no generation sets assigned");
                return;
            }
            // The camera pose's Z anchor: the draw sets (lateral,
            // height) absolutely and keeps the authored Z, which the
            // frustum fit then compensates along the view axis.
            var cameraBaseZ = camera.transform.position.z;
            foreach (var set in m_Sets)
            {
                if (set == null)
                {
                    continue;
                }
                var runner = new GenerationSetRunner(
                    set,
                    m_Rig,
                    instance.gameObject,
                    camera,
                    m_DirectionalLight,
                    m_RobotBounds,
                    m_KeepBoundsInFrustrum,
                    cameraBaseZ);
                runner.Run();
            }

#if UNITY_EDITOR
            // The run is the whole play session: once every set's data is
            // written, play mode exits itself. ExitPlaymode is the
            // editor's own API for the switch; deferred through
            // delayCall so the exit lands after this script call stack
            // returns to the editor.
            UnityEditor.EditorApplication.delayCall += () =>
                UnityEditor.EditorApplication.ExitPlaymode();
#endif
        }

        // The assembly instance, matched against the rig's STEP asset
        // details: in the editor a scene root whose prefab source is the
        // stored STEP path (the reference's path backfills while the
        // string is empty), else a root named after the STEP asset.
        // Active scene only.
        private Transform FindStepAssetInstance()
        {
            var stepPath = m_Rig.StepAssetPath;
            var stepName = m_Rig.StepAsset != null
                ? m_Rig.StepAsset.name
                : System.IO.Path.GetFileNameWithoutExtension(stepPath);
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
            Debug.LogWarning(
                $"[SyntheticDataGenerator] no instance of '{(stepPath ?? stepName)}' found in the active scene");
            return null;
        }
    }
}
