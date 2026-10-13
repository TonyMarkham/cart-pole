using UnityEngine;

namespace CartPole.Vision
{
    // One forced rig state: the cart position (meters along the rail)
    // and the pole rotation (radians about the hinge) applied verbatim
    // at the start of every randomizer run, so the hard cases are always
    // in the dataset.
    [CreateAssetMenu(menuName = "Cart-Pole/Forced Situation", fileName = "Situation")]
    public sealed class ForcedSituationSO : ScriptableObject
    {
        public string label;

        public float cartPosition;

        public float poleRotation;

        // The capture camera position for this situation: with
        // useCameraPosition on, the capture camera is placed exactly
        // here and jitter is skipped for that frame - the situation
        // reproduces the framed shot verbatim. Off by default, so a
        // situation without camera intent never moves the camera.
        public bool useCameraPosition;

        public Vector3 cameraPosition;

        // The camera orientation: recorded by the manifest (euler) and
        // applied verbatim with the position - a shot is position AND
        // orientation, and the jittered LookAt aim of the source frame
        // is not recoverable from the position alone.
        public Vector3 cameraRotation;

        // Full-shot reproduction: with a use-flag on, the situation also
        // forces that scene element (desk, skybox, light) instead of the
        // run's randomized values.
        public bool useDesk;

        public DeskSO desk;

        public bool useSkybox;

        public Material skybox;

        public float skyboxYaw;

        public bool useLight;

        public float lightIntensity;

        public float lightYaw;
    }
}
