using System.Collections.Generic;
using Unity.Mathematics;
using UnityEngine;

// ReSharper disable InconsistentNaming
// ReSharper disable UnusedMember.Global

namespace Simulation.CartPole.SyntheticDataGen
{
    // One generation run's configuration. The seed drives every random
    // choice of the run (rig states, desk/skybox/light randomization) -
    // the same seed replays the same dataset. The output directory is
    // where the run's dataset folder is written; it lives outside the
    // Unity project (dataset bytes are not Assets), so it is a plain
    // path string, never an asset reference.
    [CreateAssetMenu(menuName = MenuManager.GenerationSetMenuName, fileName = MenuManager.GenerationSetFileName)]
    public sealed class GenerationSetSO : ScriptableObject
    {
        [SerializeField] private int m_Seed;
        public int Seed => m_Seed;

        [SerializeField] private int m_States;
        public int States => m_States;
        
        [SerializeField] private string m_OutputDirectory;
        public string OutputDirectory => m_OutputDirectory;

        [SerializeField] private int m_CameraImageWidth;
        public int CameraImageWidth => m_CameraImageWidth;
        [SerializeField] private int m_CameraImageHeight;
        public int CameraImageHeight => m_CameraImageHeight;
        
        [SerializeField] private bool m_RandomizeCameraYaw = true;
        public bool RandomizeCameraYaw => m_RandomizeCameraYaw;
        [SerializeField] private float2 m_CameraYawRange = new float2(-5.0f, 5.0f);
        public float2 CameraYawRange => m_CameraYawRange;
        [SerializeField] private bool m_RandomizeCameraPitch = true;
        public bool RandomizeCameraPitch => m_RandomizeCameraPitch;
        [SerializeField] private float2 m_CameraPitchRange = new float2(-5.0f, 5.0f);
        public float2 CameraPitchRange => m_CameraPitchRange;
        [SerializeField] private bool m_RandomizeCameraHeight = true;
        public bool RandomizeCameraHeight => m_RandomizeCameraHeight;
        [SerializeField] private float2 m_CameraHeightRange = new float2(0.125f, 0.375f);
        public float2 CameraHeightRange => m_CameraHeightRange;
        [SerializeField] private bool m_RandomizeCameraLateral = true;
        public bool RandomizeCameraLateral => m_RandomizeCameraLateral;
        [SerializeField] private float2 m_CameraLateralRange = new float2(-0.125f, 0.125f);
        public float2 CameraLateralRange => m_CameraLateralRange;

        // The frustum-compensation cap, in meters: the Z dolly that fits
        // the RobotBounds back into frame after the random camera
        // transforms. A draw whose compensation exceeds it is not
        // captured and does not count toward States - the generator
        // moves to the next draw, so the set still yields its full
        // States captures. Infinity (the default) caps nothing until
        // the set authors a value.
        [SerializeField] private float m_MaxZ = float.PositiveInfinity;
        public float MaxZ => m_MaxZ;
        
        [Header("Desks")]
        [SerializeField] private List<DeskSO> m_Desks;
        public List<DeskSO> Desks => m_Desks;
        [SerializeField] private bool m_RandomizeDesks = true;
        public bool RandomizeDesks => m_RandomizeDesks;
        
        [Header("Sky-boxes")]
        [SerializeField] private List<Material> m_Skyboxes;
        public List<Material> Skyboxes => m_Skyboxes;
        [SerializeField] private bool m_RandomizeSkyboxes = true;
        public bool RandomizeSkyboxes => m_RandomizeSkyboxes;
        [SerializeField] private float2 m_SkyboxYawRange =  new float2(0.0f, 360.0f);
        public float2 SkyboxYawRange => m_SkyboxYawRange;
        
        [Header("Lighting")]
        [SerializeField] private bool m_RandomizeLightYaw = true;
        public bool RandomizeLightYaw => m_RandomizeLightYaw;
        [SerializeField] private float2 m_LightYawRange = new float2(-60.0f, 60.0f);
        public float2 LightYawRange => m_LightYawRange;
        [SerializeField] private bool m_RandomizeLightPitch = true;
        public bool RandomizeLightPitch => m_RandomizeLightPitch;
        [SerializeField] private float2 m_LightPitchRange = new float2(5.0f, 85.0f);
        public float2 LightPitchRange => m_LightPitchRange;
        [SerializeField] private bool m_RandomizeLightIntensity = true;
        public bool RandomizeLightIntensity => m_RandomizeLightIntensity;
        [SerializeField] private float2 m_LightIntensityRange = new float2(2.0f, 4.0f);
        public float2 LightIntensityRange => m_LightIntensityRange;
    }
}
