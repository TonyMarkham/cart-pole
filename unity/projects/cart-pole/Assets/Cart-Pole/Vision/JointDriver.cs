using System;
using System.Collections.Generic;
using Greyhound.Step;
using UnityEngine;

namespace CartPole.Vision
{
    // Applies gym joint states (qpos) to the imported STEP assembly.
    //
    // Each entry maps a gym-facing joint name to a StepJoint asset. The part
    // node is resolved through the joint's sibling-index path (same mechanism
    // the MJCF exporter uses), then driven in MJCF semantics:
    //   slide -> translate the part along its local axis by qpos (meters)
    //   hinge -> rotate the part about the local axis through the joint
    //            pivot by qpos (radians)
    // Base poses are captured once from the assembled (qpos = 0) state, and
    // all math runs in world space so importer scale never leaks in.
    public sealed class JointDriver : MonoBehaviour
    {
        [Serializable]
        public sealed class Entry
        {
            public string jointName;   // gym-facing name, matches MJCF <joint name>
            public StepJoint joint;    // authoring asset (Greyhound.Step)
            public Transform part;     // resolved node; auto-filled from the sibling path
        }

        public Transform assemblyRoot;   // root of the imported STEP assembly
        public List<Entry> entries = new List<Entry>();

        private readonly Dictionary<string, Entry> byName = new Dictionary<string, Entry>();

        private sealed class BasePose
        {
            public Transform node;
            public Vector3 position;
            public Quaternion rotation;
            public Vector3 axis;      // world-space joint axis
            public Vector3 pivot;     // world-space joint pivot
        }

        private readonly List<BasePose> poses = new List<BasePose>();

        [ContextMenu("Capture Base Pose")]
        public void CaptureBasePose()
        {
            byName.Clear();
            poses.Clear();
            foreach (var entry in entries)
            {
                if (entry.joint == null || string.IsNullOrEmpty(entry.jointName))
                {
                    continue;
                }
                var node = ResolvePart(entry.joint);
                if (node == null)
                {
                    Debug.LogWarning($"[JointDriver] no node resolved for '{entry.jointName}'");
                    continue;
                }
                entry.part = node;
                var axis = entry.joint.mj.axis;
                if (axis.sqrMagnitude < 1e-8f)
                {
                    axis = Vector3.right;
                }
                var pose = new BasePose
                {
                    node = node,
                    position = node.position,
                    rotation = node.rotation,
                    axis = node.TransformDirection(axis.normalized),
                    pivot = node.TransformPoint(entry.joint.mj.pos),
                };
                poses.Add(pose);
                byName[entry.jointName] = entry;
            }
        }

        private Transform ResolvePart(StepJoint joint)
        {
            if (assemblyRoot == null || joint.body == null ||
                joint.body.indices == null || joint.body.indices.Length == 0)
            {
                return null;
            }
            var t = assemblyRoot;
            foreach (var index in joint.body.indices)
            {
                if (t == null || index < 0 || index >= t.childCount)
                {
                    return null;
                }
                t = t.GetChild(index);
            }
            return t;
        }

        // qpos: joint name -> position (slide, meters) or angle (hinge, radians)
        public void Apply(IReadOnlyDictionary<string, float> qpos)
        {
            if (poses.Count == 0)
            {
                CaptureBasePose();
            }
            foreach (var pair in qpos)
            {
                if (!byName.TryGetValue(pair.Key, out var entry) || entry.part == null)
                {
                    continue;
                }
                var pose = poses.Find(p => p.node == entry.part);
                if (pose == null)
                {
                    continue;
                }
                switch (entry.joint.mj.type)
                {
                    case StepJointType.Slide:
                        entry.part.position = pose.position + pose.axis * pair.Value;
                        break;
                    case StepJointType.Hinge:
                    default:
                        var q = Quaternion.AngleAxis(pair.Value * Mathf.Rad2Deg, pose.axis);
                        entry.part.rotation = q * pose.rotation;
                        entry.part.position = pose.pivot + q * (pose.position - pose.pivot);
                        break;
                }
            }
        }

        // Ranges for random sampling: limited joints use their authored
        // limits; unlimited joints fall back to the provided span.
        public void GetRange(Entry entry, float fallbackSpan, out float lo, out float hi)
        {
            if (entry.joint != null && entry.joint.mj.limited)
            {
                lo = entry.joint.mj.rangeLo;
                hi = entry.joint.mj.rangeHi;
                return;
            }
            lo = -fallbackSpan;
            hi = fallbackSpan;
        }

        private void Start()
        {
            CaptureBasePose();
        }
    }
}
