using System;
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

namespace CartPole.Vision
{
    // Test vehicle for the rig-state pipeline: on run, applies X random
    // rig states at an interval, each pushed through the MarkerManager's
    // JSON path (ApplyState(string)) so the payload round-trip is
    // exercised, not just the float overload. Ranges mirror the authored
    // joint data: the cart's slider is limited to +-0.45 m, the pole's
    // hinge is unlimited, so its span is a parameter. With the desks
    // toggle on, each state spawns a random desk from the registered
    // DeskSO pool with the SO's placement modifiers applied - so every
    // capture also carries a randomized background prop. The skybox
    // toggle works the same way over the panoramic HDRI materials the
    // HdriSetup menu created: one is applied per state, so backgrounds
    // randomize alongside the props.
    public sealed class RigStateRandomizer : MonoBehaviour
    {
        [SerializeField] private MarkerManager markerManager;

        [SerializeField] private List<DeskSO> desks = new List<DeskSO>();

        [SerializeField] private bool randomizeDesks = true;

        [SerializeField] private List<Material> skyboxes = new List<Material>();

        [SerializeField] private bool randomizeSkybox = true;

        // The skybox material's _Rotation turns the HDRI about Unity's Y
        // axis - even the same HDRI presents a different view per state.
        [SerializeField] private float minSkyboxYaw = 0f;

        [SerializeField] private float maxSkyboxYaw = 360f;

        // Light randomization: intensity within [min, max] and a random
        // yaw (±180°) around the light's original orientation, applied
        // per state.
        [SerializeField] private bool randomizeLight = true;

        [SerializeField] private Light directionalLight;

        [SerializeField] private float minLightIntensity = 2f;

        [SerializeField] private float maxLightIntensity = 4f;

        // Front-lighting guarantee: the sun heading stays within
        // frontLightRange (degrees) of the camera-to-robot direction, so
        // the light always comes from the camera hemisphere and the rig
        // is never back-lit.
        [SerializeField] private float frontLightRange = 90f;

        [SerializeField] private int states = 100;

        [SerializeField] private float interval = 0.5f;

        [SerializeField] private float cartRange = 0.45f;

        [SerializeField] private float poleSpan = 3.0f;

        [SerializeField] private int seed;

        // Forced situations applied verbatim before the random states -
        // every run contains them, so the hard cases are always in the
        // dataset.
        [SerializeField] private List<ForcedSituationSO> forcedSituations =
            new List<ForcedSituationSO>();

        // Fired after each state is applied, carrying what the capture
        // component needs for its manifest: the state, the situation
        // label (empty when random), the spawned desk and the applied
        // skybox with its yaw.
        public event Action<StateEvent> StateApplied;

        // Fired once when the run completes (forced + random states
        // done) - the capture component kicks off the OpenCV label
        // verification on the dataset here.
        public event Action RunCompleted;

        public sealed class StateEvent
        {
            public string situation;
            public float cartPosition;
            public float poleRotation;
            public bool hasCameraPosition;
            public Vector3 cameraPosition;
            public Vector3 cameraRotation;
            public bool useDesk;
            public DeskSO deskAsset;
            public bool useSkybox;
            public Material skyboxAsset;
            public float skyboxYaw;
            public bool useLight;
            public float lightIntensity;
            public float lightYaw;
        }

        private System.Random random;
        private GameObject deskInstance;
        private string lastDeskLabel = "";
        private DeskSO lastDeskAsset;
        private Material lastSkyboxAsset;
        private bool lastDeskApplied;
        private bool lastSkyboxApplied;
        private bool lastLightApplied;
        private float lastSkyboxYaw;
        private float lastLightIntensity;
        private float lastLightYaw;
        private Quaternion lightBaseRotation;
        private float lightBaseIntensity = -1f;
        private bool warnedNoDesks;
        private bool warnedNoSkyboxes;
        private bool warnedNoLight;

        private void Start()
        {
            if (markerManager == null)
            {
                markerManager = GetComponent<MarkerManager>();
            }
            if (markerManager == null)
            {
                Debug.LogWarning("[RigStateRandomizer] no MarkerManager assigned");
                return;
            }
            random = seed == 0 ? new System.Random() : new System.Random(seed);
            StartCoroutine(RandomizeLoop());
        }

        // On play exit the spawned desk is destroyed; if it is still the
        // selected object, the inspector re-enables against the corpse
        // after the domain reload and throws - deselect it first, while
        // it is still alive.
        private void OnDisable()
        {
#if UNITY_EDITOR
            if (deskInstance != null &&
                UnityEditor.Selection.activeObject == deskInstance)
            {
                UnityEditor.Selection.activeObject = null;
            }
#endif
        }

        private IEnumerator RandomizeLoop()
        {
            // One frame of grace so the MarkerManager's Start has applied
            // the markers before the first state (and its capture) fires.
            yield return null;
            foreach (var situation in forcedSituations)
            {
                if (situation == null)
                {
                    continue;
                }
                yield return ApplyOneState(situation);
            }
            for (var i = 0; i < states; i++)
            {
                var cart = (float)(random.NextDouble() * 2 - 1) * cartRange;
                var pole = (float)(random.NextDouble() * 2 - 1) * poleSpan;
                yield return ApplyRandomState(cart, pole);
            }
            Debug.Log(
                $"[RigStateRandomizer] done - {forcedSituations.Count} forced + " +
                $"{states} random states applied");
            RunCompleted?.Invoke();
        }

        private IEnumerator ApplyOneState(ForcedSituationSO situation)
        {
            var json = JsonUtility.ToJson(new RigState
            {
                cartPosition = situation.cartPosition,
                poleRotation = situation.poleRotation,
            });
            Debug.Log($"[RigStateRandomizer] state [{situation.label}]: {json}");
            yield return RunState(
                situation.label,
                situation.cartPosition,
                situation.poleRotation,
                situation.useCameraPosition,
                situation.cameraPosition,
                situation.cameraRotation,
                json);
        }

        private IEnumerator ApplyRandomState(float cart, float pole)
        {
            var json = JsonUtility.ToJson(new RigState
            {
                cartPosition = cart,
                poleRotation = pole,
            });
            Debug.Log($"[RigStateRandomizer] state: {json}");
            yield return RunState("", cart, pole, false, Vector3.zero, Vector3.zero, json);
        }

        // The StateEvent is built here - after this state's desk, skybox
        // and light have been applied - so the manifest records what is
        // actually in the frame, not the state of the previous one.
        private IEnumerator RunState(
            string situation, float cart, float pole, bool hasCamera,
            Vector3 cameraPosition, Vector3 cameraRotation, string json)
        {
            ApplyLight();
            ApplySkybox();
            SpawnDesk();
            var stateEvent = new StateEvent
            {
                situation = situation,
                cartPosition = cart,
                poleRotation = pole,
                hasCameraPosition = hasCamera,
                cameraPosition = cameraPosition,
                cameraRotation = cameraRotation,
                useDesk = lastDeskApplied,
                deskAsset = lastDeskAsset,
                useSkybox = lastSkyboxApplied,
                skyboxAsset = lastSkyboxAsset,
                skyboxYaw = lastSkyboxYaw,
                useLight = lastLightApplied,
                lightIntensity = lastLightIntensity,
                lightYaw = lastLightYaw,
            };
            markerManager.ApplyState(json);
            StateApplied?.Invoke(stateEvent);
            yield return new WaitForSeconds(interval);
        }

        // The light for this state: intensity within [min, max] and a
        // random yaw within ±180° of the light's original orientation.
        // The base is captured on first application, so the authored sun
        // stays the anchor the randomization wanders around.
        private void ApplyLight()
        {
            lastLightApplied = false;
            if (!randomizeLight)
            {
                return;
            }
            if (directionalLight == null)
            {
                if (!warnedNoLight)
                {
                    warnedNoLight = true;
                    Debug.LogWarning(
                        "[RigStateRandomizer] randomize light is on but no directional light is assigned");
                }
                return;
            }
            if (lightBaseIntensity < 0f)
            {
                lightBaseRotation = directionalLight.transform.rotation;
                lightBaseIntensity = directionalLight.intensity;
            }
            var intensity = minLightIntensity +
                (float)random.NextDouble() * (maxLightIntensity - minLightIntensity);
            var yaw = FrontYaw();
            directionalLight.intensity = intensity;
            var baseEuler = lightBaseRotation.eulerAngles;
            directionalLight.transform.rotation =
                Quaternion.Euler(baseEuler.x, yaw, baseEuler.z);
            lastLightIntensity = intensity;
            lastLightYaw = yaw;
            lastLightApplied = true;
        }

        // The sun's world yaw: the camera-to-robot heading plus a random
        // offset within frontLightRange - the light always comes from
        // the camera hemisphere, so the rig is never back-lit.
        private float FrontYaw()
        {
            var subject = markerManager != null
                ? markerManager.ResolveSceneRoot()
                : null;
            var cam = Camera.main;
            if (subject == null || cam == null)
            {
                return (float)(random.NextDouble() * 2 - 1) * 180f;
            }
            var toSubject = subject.position - cam.transform.position;
            toSubject.y = 0f;
            if (toSubject.sqrMagnitude < 1e-6f)
            {
                return (float)(random.NextDouble() * 2 - 1) * 180f;
            }
            var heading = Vector3.SignedAngle(Vector3.forward, toSubject, Vector3.up);
            return heading + (float)(random.NextDouble() * 2 - 1) * frontLightRange;
        }

        // The skybox for this state: a random entry from the panoramic
        // HDRI materials with a random yaw about Unity's Y axis (the
        // material's _Rotation), applied through the scene's
        // RenderSettings so the capture camera sees it. Not restored on
        // toggle-off - the last applied skybox simply stays.
        private void ApplySkybox()
        {
            lastSkyboxApplied = false;
            if (!randomizeSkybox)
            {
                return;
            }
            if (skyboxes == null || skyboxes.Count == 0)
            {
                if (!warnedNoSkyboxes)
                {
                    warnedNoSkyboxes = true;
                    Debug.LogWarning(
                        "[RigStateRandomizer] randomize skybox is on but the pool is empty");
                }
                return;
            }
            var skybox = skyboxes[random.Next(skyboxes.Count)];
            var yaw = minSkyboxYaw + (float)random.NextDouble() * (maxSkyboxYaw - minSkyboxYaw);
            skybox.SetFloat("_Rotation", yaw);
            RenderSettings.skybox = skybox;
            lastSkyboxAsset = skybox;
            lastSkyboxApplied = true;
            lastSkyboxYaw = yaw;
        }

        // The desk for this state: a random entry from the registered
        // pool, spawned with the SO's placement modifiers, replacing the
        // previous instance. DestroyImmediate covers the edit-mode
        // Randomize Once path as well as play mode. With the toggle off
        // the current desk is removed, so captures are desk-free.
        private void SpawnDesk()
        {
            lastDeskApplied = false;
            if (!randomizeDesks)
            {
                if (deskInstance != null)
                {
                    DestroyImmediate(deskInstance);
                    deskInstance = null;
                }
                lastDeskLabel = "";
                lastDeskAsset = null;
                return;
            }
            if (desks == null || desks.Count == 0)
            {
                if (!warnedNoDesks)
                {
                    warnedNoDesks = true;
                    Debug.LogWarning(
                        "[RigStateRandomizer] randomize desks is on but the pool is empty");
                }
                return;
            }
            var desk = desks[random.Next(desks.Count)];
            if (desk.prefab == null)
            {
                Debug.LogWarning($"[RigStateRandomizer] desk '{desk.label}' has no prefab");
                return;
            }
            if (deskInstance != null)
            {
                // A destroyed object that is still selected throws the
                // inspector's MissingReferenceException on its next
                // OnEnable - deselect it before the kill.
#if UNITY_EDITOR
                if (UnityEditor.Selection.activeGameObject == deskInstance)
                {
                    UnityEditor.Selection.activeGameObject = null;
                }
#endif
                DestroyImmediate(deskInstance);
            }
            deskInstance = Instantiate(
                desk.prefab, desk.position, Quaternion.Euler(desk.localEuler));
            deskInstance.transform.localScale = desk.scale;
            deskInstance.name = $"Desk_{desk.label}";
            lastDeskLabel = desk.label;
            lastDeskAsset = desk;
            lastDeskApplied = true;
        }

        // Single random state in edit mode for a quick visual check.
        [ContextMenu("Randomize Once")]
        private void RandomizeOnce()
        {
            if (markerManager == null)
            {
                markerManager = GetComponent<MarkerManager>();
            }
            if (markerManager == null)
            {
                Debug.LogWarning("[RigStateRandomizer] no MarkerManager assigned");
                return;
            }
            random ??= seed == 0 ? new System.Random() : new System.Random(seed);
            var cart = (float)(random.NextDouble() * 2 - 1) * cartRange;
            var pole = (float)(random.NextDouble() * 2 - 1) * poleSpan;
            ApplySkybox();
            SpawnDesk();
            markerManager.ApplyState(JsonUtility.ToJson(new RigState
            {
                cartPosition = cart,
                poleRotation = pole,
            }));
            Debug.Log($"[RigStateRandomizer] cart={cart:0.000} pole={pole:0.000} rad");
        }

        // The payload shape MarkerManager parses; kept identical to the
        // runtime contract so the test pushes exactly what the gym will.
        [Serializable]
        private sealed class RigState
        {
            public float cartPosition;
            public float poleRotation;
        }
    }
}
