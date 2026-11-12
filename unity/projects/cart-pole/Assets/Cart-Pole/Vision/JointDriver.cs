using System;
using System.Collections.Generic;
using Greyhound.MuJoCo;
using Greyhound.Utilities;
using UnityEngine;

// UnityEngine also has a `Joint` (the physics component), and the MuJoCo
// asset and its payload are both named Joint: the alias names the asset.
using JointAsset = Greyhound.MuJoCo.ScriptableObjects.Joint;

namespace CartPole.Vision
{
    // Applies gym joint states (qpos) to the imported STEP assembly.
    //
    // Each entry maps a gym-facing joint name to a MuJoCo Joint asset
    // (com.greyhound.mujoco). The part node is resolved through the joint's
    // sibling-index path (`body`, the same path the MJCF exporter writes),
    // then driven in MJCF semantics:
    //   slide -> translate the part along its local axis by qpos (meters)
    //   hinge -> rotate the part about the local axis through the joint
    //            pivot by qpos (radians)
    // Free and ball joints take more than one qpos value, so a single
    // float cannot drive them; they are skipped.
    // Base poses are captured once from the assembled (qpos = 0) state, and
    // all math runs in world space so importer scale never leaks in.
    public sealed class JointDriver : MonoBehaviour
    {
        [Serializable]
        public sealed class Entry
        {
            public string jointName;   // gym-facing name, matches MJCF <joint name>
            public JointAsset joint;   // authoring asset (Greyhound.MuJoCo)
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
                var axis = entry.joint.joint.axis;
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
                    pivot = node.TransformPoint(entry.joint.joint.pos),
                };
                poses.Add(pose);
                byName[entry.jointName] = entry;
            }
        }

        // Walks `body.indices` down from assemblyRoot. [SiblingPath.SELF]
        // ([-1]) is assemblyRoot itself; [2, 0] is
        // assemblyRoot.GetChild(2).GetChild(0); an empty path names no part.
        private Transform ResolvePart(JointAsset joint)
        {
            if (assemblyRoot == null || joint.body == null ||
                joint.body.indices == null || joint.body.indices.Length == 0)
            {
                return null;
            }
            var t = assemblyRoot;
            foreach (var index in joint.body.indices)
            {
                if (index == SiblingPath.SELF)
                {
                    continue;
                }
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
                switch (entry.joint.joint.type)
                {
                    case JointType.Slide:
                        entry.part.position = pose.position + pose.axis * pair.Value;
                        break;
                    case JointType.Hinge:
                        var q = Quaternion.AngleAxis(pair.Value * Mathf.Rad2Deg, pose.axis);
                        entry.part.rotation = q * pose.rotation;
                        entry.part.position = pose.pivot + q * (pose.position - pose.pivot);
                        break;
                }
            }
        }

        // Ranges for random sampling: limited joints use their authored
        // limits; unlimited joints fall back to the provided span.
        // "Limited" is limited = True, the same rule the MJCF exporter uses
        // to decide whether it writes `range` at all (Auto writes no range,
        // so MuJoCo treats the joint as unlimited).
        public void GetRange(Entry entry, float fallbackSpan, out float lo, out float hi)
        {
            if (entry.joint != null && entry.joint.joint.limited == FalseTrueAuto.True)
            {
                lo = entry.joint.joint.rangeLo;
                hi = entry.joint.joint.rangeHi;
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
