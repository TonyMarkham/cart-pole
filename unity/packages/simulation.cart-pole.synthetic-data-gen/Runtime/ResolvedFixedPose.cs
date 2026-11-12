using System.Collections.Generic;

using Greyhound.MuJoCo;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One fixed pose of a generation set with its names looked up
    // (ResolvedGenerationSet): each listed joint's driver with its value in
    // MJCF qpos units - meters for a slide, radians for a hinge, though the
    // set writes a hinge in degrees - and each prop's PropPoseSO from the
    // catalog. Example: {"joint": "04-Pole.Hinge.joint", "value": -180}
    // becomes the pole hinge's driver with -3.1415927.
    internal sealed class ResolvedFixedPose
    {
        private readonly IReadOnlyList<(JointDriver Driver, float Qpos)> m_Joints;

        internal ResolvedFixedPose(
            string name,
            IReadOnlyList<(JointDriver Driver, float Qpos)> joints,
            IReadOnlyList<PropPoseSO> props)
        {
            Name = name;
            m_Joints = joints;
            Props = props;
        }

        // For the logs only.
        internal string Name { get; }

        // Placed after the robot takes the pose and removed after the
        // frame, so they appear in this pose's frames only.
        internal IReadOnlyList<PropPoseSO> Props { get; }

        // Every driver in `drivers` back to rest, then each listed joint to
        // its value: a joint the pose leaves out stays at its imported pose.
        internal void Apply(IReadOnlyList<JointDriver> drivers)
        {
            foreach (var driver in drivers)
            {
                driver.ResetPose();
            }
            foreach (var (driver, qpos) in m_Joints)
            {
                driver.Apply(qpos);
            }
        }
    }
}
