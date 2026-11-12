using System;

using Greyhound.MuJoCo;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One joint's value in a FixedPoseSO, in MJCF's sense: meters for a
    // slide, degrees for a hinge (degrees because poses are authored by
    // eye). Example: the pole hinge at -180 is the pole straight up.
    //
    // Was the nested type FixedPoseSO.JointValue. Unity serializes a
    // [Serializable] class by its field names (`joint`, `value`), not its
    // type name, so fixed poses saved under the old type load unchanged.
    [Serializable]
    public sealed class FixedPoseJointValue
    {
        public Greyhound.MuJoCo.ScriptableObjects.Joint joint;

        // Meters for a slide, degrees for a hinge (MJCF sense).
        public float value;

        // The value in MJCF qpos units: meters, or radians for a hinge.
        public float Qpos(JointType type) =>
            type == JointType.Hinge ? value * Mathf.Deg2Rad : value;
    }
}
