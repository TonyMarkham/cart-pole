using System;
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
        // markers applied it runs every assigned set - play mode only
        // (editor-mode generation is rejected: it makes crap data), one
        // set after another, each writing its own
        // run_<set>_<timestamp> folder under its output directory. The
        // run is async: each state is a synchronous main-thread chunk,
        // and the awaits between states hand the player loop back to
        // the editor.
        private async void Start()
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
            // At least two sets: data.yaml's last set is val and every
            // earlier one is train, so one set would leave train empty.
            var assignedSets = 0;
            foreach (var set in m_Sets ?? new List<GenerationSetSO>())
            {
                if (set != null)
                {
                    assignedSets++;
                }
            }
            if (assignedSets < 2)
            {
                Debug.LogWarning(
                    $"[SyntheticDataGenerator] {assignedSets} generation set(s) assigned - " +
                    "at least 2 are needed: the last is val, the rest are train");
                return;
            }
            // The camera's authored pose, read once before any set runs:
            // every draw offsets it (lateral, height, pitch, yaw) and the
            // frustum fit then compensates along the view axis.
            var cameraBasePosition = camera.transform.position;
            var cameraBaseRotation = camera.transform.rotation;
            var runDirectories = new List<string>();
            GenerationSetSO lastSet = null;
            string emptyRun = null;
            try
            {
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
                        cameraBasePosition,
                        cameraBaseRotation);
                    await runner.RunAsync();
                    if (runner.Frames == 0)
                    {
                        emptyRun = runner.RunDirectory;
                    }
                    runDirectories.Add(runner.RunDirectory);
                    lastSet = set;
                }
                // Every set has written its run, so the yaml lists exactly
                // this generation; it lands beside the runs, in the last
                // set's output directory. An exception above skips it, and
                // so does a set that wrote no frames: its empty folder
                // would be listed, possibly as val.
                if (emptyRun != null)
                {
                    Debug.LogError(
                        $"[SyntheticDataGenerator] {emptyRun} has no frames - data.yaml not written");
                }
                else
                {
                    DatasetWriter.WriteDataYaml(lastSet.OutputDirectory, runDirectories, m_Rig);
                    Debug.Log(
                        $"[SyntheticDataGenerator] data.yaml: {runDirectories.Count - 1} train run(s), " +
                        $"val {System.IO.Path.GetFileName(runDirectories[^1])} -> {lastSet.OutputDirectory}");
                }
            }
            catch (Exception exception)
            {
                // An async void entry point: an unhandled exception
                // here would kill the run silently to the player loop.
                Debug.LogException(exception);
            }

#if UNITY_EDITOR
            // The run is the whole play session: once every set's data is
            // written, play mode exits itself. ExitPlaymode is the
            // editor's own API for the switch; deferred through
            // delayCall so the exit lands after this call stack returns
            // to the editor.
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
