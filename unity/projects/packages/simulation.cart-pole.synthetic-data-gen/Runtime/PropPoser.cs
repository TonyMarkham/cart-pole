using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // A setup aid: on entering play mode it puts the scene's robot in a
    // fixed pose and places that pose's props, exactly as the generator
    // does for a fixed-pose frame (RigSO.ApplyState, then each
    // PropPoseSO.Place), so a pose and its props can be checked by eye
    // before a generation set uses them. Everything it places exists
    // only while the scene plays.
    public sealed class PropPoser : MonoBehaviour
    {
        // The rig whose cart and pole paths the pose drives.
        [SerializeField] private RigSO m_Rig;
        public RigSO Rig => m_Rig;

        // The scene instance of the rig's STEP assembly - dragged from the
        // Hierarchy, not the Project window: props cannot be parented
        // inside an asset.
        [SerializeField] private GameObject m_Robot;
        public GameObject Robot => m_Robot;

        [SerializeField] private FixedPoseSO m_FixedPose;
        public FixedPoseSO FixedPose => m_FixedPose;

        private void Start()
        {
            if (m_Rig == null || m_Robot == null || m_FixedPose == null)
            {
                Debug.LogWarning(
                    "[PropPoser] assign the Rig, the Robot and the Fixed Pose in the inspector",
                    this);
                return;
            }
            // An asset has no scene; a scene instance always has one.
            if (!m_Robot.scene.IsValid())
            {
                Debug.LogWarning(
                    $"[PropPoser] Robot '{m_Robot.name}' is an asset - drag its " +
                    "instance from the Hierarchy instead",
                    this);
                return;
            }
            m_Rig.ApplyState(m_Robot, m_FixedPose.CartPosition, m_FixedPose.PoleRadians);
            var placed = 0;
            foreach (var propPose in m_FixedPose.Props)
            {
                if (propPose != null && propPose.Place(m_Robot) != null)
                {
                    placed++;
                }
            }
            Debug.Log(
                $"[PropPoser] '{m_FixedPose.name}': cart {m_FixedPose.CartPosition} m, " +
                $"pole {m_FixedPose.PoleDegrees} deg, {placed}/{m_FixedPose.Props.Count} props placed",
                this);
        }
    }
}
