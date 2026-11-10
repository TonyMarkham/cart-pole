using System.Collections.Generic;

using UnityEngine;
using UnityEngine.Serialization;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One fixed rig state with the props that belong to it: a frame the
    // generator captures on purpose instead of drawing at random - the
    // cart at a limit switch with the pole up and a T-square beside it,
    // say. The cart is in meters along the rail and the pole in degrees
    // about the hinge, both 0 at the assembly's imported pose;
    // PoleRadians is the pole angle in RigSO.ApplyState's unit.
    [CreateAssetMenu(menuName = MenuManager.FixedPoseMenuName, fileName = MenuManager.FixedPoseFileName)]
    public sealed class FixedPoseSO : ScriptableObject
    {
        [SerializeField] private float m_CartPosition;
        public float CartPosition => m_CartPosition;

        // Was m_PoleRotation (radians); the rename keeps the value the
        // assets already hold, which was authored meaning degrees.
        [FormerlySerializedAs("m_PoleRotation")]
        [SerializeField] private float m_PoleDegrees;
        public float PoleDegrees => m_PoleDegrees;
        public float PoleRadians => m_PoleDegrees * Mathf.Deg2Rad;

        // Placed after the rig takes the pose and removed after the
        // frame, so they appear in this pose's frame only.
        [SerializeField] private List<PropPoseSO> m_Props = new();
        public IReadOnlyList<PropPoseSO> Props => m_Props;
    }
}
