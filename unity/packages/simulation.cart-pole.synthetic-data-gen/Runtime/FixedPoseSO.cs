using System.Collections.Generic;

using Greyhound.MuJoCo;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One fixed robot state with the props that belong to it, for
    // PropPoser to set up in the scene and check by eye - the cart at a
    // limit switch with the pole up and a T-square beside it, say. (The
    // generator's fixed poses arrive in its GenerationSet instead.) The state is one value per listed joint, in MJCF's sense (the
    // value a StateServer message would carry for that joint): meters for
    // a slide, degrees for a hinge (radians in the message; degrees here
    // because poses are authored by eye). Every joint starts at 0, the
    // imported pose; a joint the pose does not list stays there.
    // Example: cart slider 0.445, pole hinge -180 is the cart at the motor
    // end with the pole up.
    [CreateAssetMenu(menuName = MenuManager.FixedPoseMenuName, fileName = MenuManager.FixedPoseFileName)]
    public sealed class FixedPoseSO : ScriptableObject
    {
        [SerializeField] private List<FixedPoseJointValue> m_Joints = new();
        public IReadOnlyList<FixedPoseJointValue> Joints => m_Joints;

        // Placed after the robot takes the pose and removed after the
        // frame, so they appear in this pose's frame only.
        [SerializeField] private List<PropPoseSO> m_Props = new();
        public IReadOnlyList<PropPoseSO> Props => m_Props;

        // Poses the robot (Play mode only, like JointDriver.Attach): every
        // driver in `drivers` back to rest, then each listed joint to its
        // value. A listed joint with no driver in `drivers` is attached
        // first, on its part below `instance` (the robot in the scene).
        // Returns the drivers it posed, for the log.
        public int Apply(IEnumerable<JointDriver> drivers, Transform instance)
        {
            var byJoint = new Dictionary<Greyhound.MuJoCo.ScriptableObjects.Joint, JointDriver>();
            foreach (var driver in drivers)
            {
                if (driver != null)
                {
                    driver.ResetPose();
                    byJoint[driver.Source] = driver;
                }
            }
            var posed = 0;
            foreach (var value in m_Joints)
            {
                if (value?.joint == null)
                {
                    continue;
                }
                if (!byJoint.TryGetValue(value.joint, out var target))
                {
                    target = JointDriver.Attach(value.joint, instance);
                }
                if (target == null)
                {
                    continue;
                }
                target.Apply(value.Qpos(target.Type));
                posed++;
            }
            return posed;
        }
    }
}
