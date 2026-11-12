using System;

using Greyhound.MuJoCo;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // A setup aid: on entering play mode it puts the scene's robot in a
    // fixed pose and places that pose's props, as the generator does for a
    // fixed-pose frame (here FixedPoseSO.Apply, which attaches a
    // JointDriver to each joint the pose lists, then each
    // PropPoseSO.Place), so a pose and its props can be checked by eye
    // before a generation set uses them. Everything it places exists
    // only while the scene plays.
    public sealed class PropPoser : MonoBehaviour
    {
        // Every log line starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[PropPoser]";

        // The scene instance of the robot's STEP assembly - dragged from the
        // Hierarchy, not the Project window: props cannot be parented
        // inside an asset.
        [SerializeField] private GameObject m_Robot;
        public GameObject Robot => m_Robot;

        [SerializeField] private FixedPoseSO m_FixedPose;
        public FixedPoseSO FixedPose => m_FixedPose;

        private void Start()
        {
            if (m_Robot == null || m_FixedPose == null)
            {
                Debug.LogWarning(
                    $"{k_LogPrefix} assign the Robot and the Fixed Pose in the inspector",
                    this);
                return;
            }
            // An asset has no scene; a scene instance always has one.
            if (!m_Robot.scene.IsValid())
            {
                Debug.LogWarning(
                    $"{k_LogPrefix} Robot '{m_Robot.name}' is an asset - drag its " +
                    "instance from the Hierarchy instead",
                    this);
                return;
            }
            var posed = m_FixedPose.Apply(Array.Empty<JointDriver>(), m_Robot.transform);
            var placed = 0;
            foreach (var propPose in m_FixedPose.Props)
            {
                if (propPose != null && propPose.Place(m_Robot) != null)
                {
                    placed++;
                }
            }
            Debug.Log(
                $"{k_LogPrefix} '{m_FixedPose.name}': {posed}/{m_FixedPose.Joints.Count} joints posed, " +
                $"{placed}/{m_FixedPose.Props.Count} props placed",
                this);
        }
    }
}
