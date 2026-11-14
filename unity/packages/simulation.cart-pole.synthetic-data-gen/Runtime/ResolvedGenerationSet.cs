using System.Collections.Generic;
using System.IO;

using Greyhound.MuJoCo;

using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // A GenerationJob turned into what a run uses: every name looked up in
    // this player, and every number checked, before a single frame renders.
    // Desks, skyboxes and props are asset names in the CatalogSO; fixed-pose
    // joints are MJCF joint names among the StateServer's drivers. A name
    // that is not there fails the whole job, naming it, rather than running
    // a set that differs from the one the generator sent.
    //
    // A range the set leaves out is null here, and null means "draw
    // nothing": a camera offset of 0, the light's or skybox's value left as
    // the scene has it, an unlimited hinge at 0. That is protobuf's rule for
    // an absent message field, and it is what the old GenerationSetSO's
    // Randomize* switches did when off.
    internal sealed class ResolvedGenerationSet
    {
        private ResolvedGenerationSet()
        {
        }

        // The run folder's name part: run_<Name>_<timestamp>.
        internal string Name { get; private set; }

        // Absolute; the run folder is written inside it.
        internal string OutputDirectory { get; private set; }

        internal int Seed { get; private set; }

        // Frames captured, fixed-pose frames included; at least 1.
        internal int States { get; private set; }

        internal IReadOnlyList<ResolvedFixedPose> FixedPoses { get; private set; }

        internal int FixedPoseFrames { get; private set; }

        // The captured frame's size in pixels; each at least 1.
        internal int CameraImageWidth { get; private set; }
        internal int CameraImageHeight { get; private set; }

        // Offsets from the camera's pose in the scene: degrees for yaw,
        // pitch and roll, meters for height (world Y), lateral (world X) and
        // distance (world Z).
        internal float2? CameraYaw { get; private set; }
        internal float2? CameraPitch { get; private set; }
        internal float2? CameraHeight { get; private set; }
        internal float2? CameraLateral { get; private set; }
        internal float2? CameraRoll { get; private set; }
        internal float2? CameraDistance { get; private set; }

        // Empty: no desk.
        internal IReadOnlyList<DeskSO> Desks { get; private set; }

        // Empty: the scene's own skybox, unturned.
        internal IReadOnlyList<Material> Skyboxes { get; private set; }

        // Degrees about the world's up axis.
        internal float2? SkyboxYaw { get; private set; }

        // Absolute world values, not offsets: degrees for yaw and pitch,
        // Unity's units for intensity.
        internal float2? LightYaw { get; private set; }
        internal float2? LightPitch { get; private set; }
        internal float2? LightIntensity { get; private set; }

        // Radians, for a hinge with no MJCF range of its own.
        internal float2? UnlimitedHinge { get; private set; }

        // The job resolved against this player's catalog and drivers; false,
        // with `error` saying which field or name failed, when any check
        // fails.
        internal static bool TryResolve(
            Proto.GenerationJob job,
            CatalogSO catalog,
            IReadOnlyList<JointDriver> drivers,
            out ResolvedGenerationSet resolved,
            out string error)
        {
            resolved = null;
            var set = job.Set;
            if (set == null)
            {
                error = "the job has no set";
                return false;
            }
            if (string.IsNullOrEmpty(set.Name))
            {
                error = "the set has no name (the run folder is named after it)";
                return false;
            }
            if (string.IsNullOrEmpty(job.OutputDirectory) || !Path.IsPathRooted(job.OutputDirectory))
            {
                error = $"set '{set.Name}': output_directory '{job.OutputDirectory}' is not an absolute path";
                return false;
            }
            if (!TryCount(set.Name, "states", set.States, 1, out var states, out error) ||
                !TryCount(set.Name, "fixed_pose_frames", set.FixedPoseFrames, 0, out var fixedPoseFrames, out error) ||
                !TryCount(set.Name, "camera_image_width", set.CameraImageWidth, 1, out var width, out error) ||
                !TryCount(set.Name, "camera_image_height", set.CameraImageHeight, 1, out var height, out error))
            {
                return false;
            }

            var desks = new List<DeskSO>();
            foreach (var deskName in set.Desks)
            {
                if (!catalog.TryGetDesk(deskName, out var desk))
                {
                    error = $"set '{set.Name}': desk '{deskName}' is not in catalog '{catalog.name}'";
                    return false;
                }
                desks.Add(desk);
            }
            var skyboxes = new List<Material>();
            foreach (var skyboxName in set.Skyboxes)
            {
                if (!catalog.TryGetSkybox(skyboxName, out var skybox))
                {
                    error = $"set '{set.Name}': skybox '{skyboxName}' is not in catalog '{catalog.name}'";
                    return false;
                }
                skyboxes.Add(skybox);
            }

            var driversByName = new Dictionary<string, JointDriver>();
            foreach (var driver in drivers)
            {
                driversByName[driver.JointName] = driver;
            }
            var fixedPoses = new List<ResolvedFixedPose>();
            foreach (var pose in set.FixedPoses)
            {
                var joints = new List<(JointDriver Driver, float Qpos)>();
                foreach (var value in pose.Joints)
                {
                    if (!driversByName.TryGetValue(value.Joint, out var driver))
                    {
                        error = $"set '{set.Name}', fixed pose '{pose.Name}': joint '{value.Joint}' " +
                                "is not a drivable joint of the StateServer's Root";
                        return false;
                    }
                    // Degrees in the set for a hinge (written by eye),
                    // radians on the driver.
                    var qpos = driver.Type == JointType.Hinge ? value.Value * Mathf.Deg2Rad : value.Value;
                    joints.Add((driver, qpos));
                }
                var props = new List<PropPoseSO>();
                foreach (var propName in pose.Props)
                {
                    if (!catalog.TryGetPropPose(propName, out var prop))
                    {
                        error = $"set '{set.Name}', fixed pose '{pose.Name}': prop pose '{propName}' " +
                                $"is not in catalog '{catalog.name}'";
                        return false;
                    }
                    props.Add(prop);
                }
                fixedPoses.Add(new ResolvedFixedPose(pose.Name, joints, props));
            }

            resolved = new ResolvedGenerationSet
            {
                Name = set.Name,
                OutputDirectory = job.OutputDirectory,
                Seed = set.Seed,
                States = states,
                FixedPoses = fixedPoses,
                FixedPoseFrames = fixedPoseFrames,
                CameraImageWidth = width,
                CameraImageHeight = height,
                CameraYaw = ToRange(set.CameraYaw),
                CameraPitch = ToRange(set.CameraPitch),
                CameraHeight = ToRange(set.CameraHeight),
                CameraLateral = ToRange(set.CameraLateral),
                CameraRoll = ToRange(set.CameraRoll),
                CameraDistance = ToRange(set.CameraDistance),
                Desks = desks,
                Skyboxes = skyboxes,
                SkyboxYaw = ToRange(set.SkyboxYaw),
                LightYaw = ToRange(set.LightYaw),
                LightPitch = ToRange(set.LightPitch),
                LightIntensity = ToRange(set.LightIntensity),
                UnlimitedHinge = ToRange(set.UnlimitedHinge),
            };
            error = null;
            return true;
        }

        // A protobuf uint32 count as an int: at least `minimum`, and no more
        // than int.MaxValue, which the loops and texture sizes use.
        private static bool TryCount(
            string setName, string field, uint value, int minimum, out int count, out string error)
        {
            if (value < minimum || value > int.MaxValue)
            {
                count = 0;
                error = $"set '{setName}': {field} is {value}; it must be from {minimum} to {int.MaxValue}";
                return false;
            }
            count = (int)value;
            error = null;
            return true;
        }

        private static float2? ToRange(Proto.Range range)
        {
            return range == null ? (float2?)null : new float2(range.Min, range.Max);
        }
    }
}
