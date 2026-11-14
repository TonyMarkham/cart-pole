using UnityEngine;
using UnityEngine.Rendering;

using SiblingPath = Greyhound.Utilities.SiblingPath;

namespace Simulation.CartPole.SyntheticDataGen
{
    // The robot workspace envelope: a plane GameObject (Unity's built-in
    // Plane, 10 x 10 m in its own XZ, normal +Y), scaled and turned to face
    // the camera. Its renderer bounds are the frustum contract the camera
    // jitter honors. Making it invisible is the renderer's checkbox;
    // captures hide it automatically even when the renderer is on, so it
    // never pollutes a frame.
    //
    // Where it sits follows the robot, not a typed number: in Awake its
    // origin is put on the origin of `m_target`, a part of the robot's
    // assembly named by a SiblingPath (the same rename-robust reference
    // the markers use). The indices are walked down from the scene's copy
    // of the path's root (see ResolveTarget). Only the position moves: the
    // plane's own rotation and scale stay as authored.
    [RequireComponent(typeof(MeshFilter), typeof(MeshRenderer))]
    public sealed class RobotBounds : MonoBehaviour
    {
        // Every log line starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[RobotBounds]";

        // The part whose origin the plane's origin is put on. Dropping the
        // part on the field (the SiblingPath drawer) sets the path's root
        // and indices.
        [SerializeField] private SiblingPath m_target = new();

        [SerializeField] private Transform m_TopLeft;
        [SerializeField] private Transform m_TopRight;
        [SerializeField] private Transform m_BottomLeft;
        [SerializeField] private Transform m_BottomRight;

        public Transform TopLeft => m_TopLeft;
        public Transform TopRight => m_TopRight;
        public Transform BottomLeft => m_BottomLeft;
        public Transform BottomRight => m_BottomRight;

        public Bounds Bounds => GetComponent<Renderer>().bounds;

        // The plane's world position becomes the target's world position;
        // its parent is never changed. If the path does not resolve, the
        // plane stays where it was authored and the reason is logged.
        private void Awake()
        {
            var target = ResolveTarget();
            if (target == null)
            {
                return;
            }
            transform.position = target.position;
        }

        // The part the path names, found in the scene's copy of the robot,
        // or null (with the reason logged as an error).
        //
        // The path's root is the STEP model asset, not its copy in the
        // scene: walking the indices from the asset finds the part where it
        // sits in the asset, without the scene's placement (measured: the
        // plane landed 1 m low, the robot's local y under PlayerController).
        // So the indices are walked from the scene's copy instead: the
        // sibling of this plane (a child of the same parent) that has the
        // asset's name.
        private Transform ResolveTarget()
        {
            if (m_target.root == null)
            {
                Debug.LogError($"{k_LogPrefix} target path has no root", this);
                return null;
            }
            if (m_target.indices == null || m_target.indices.Length == 0)
            {
                Debug.LogError($"{k_LogPrefix} no target path set", this);
                return null;
            }
            var instance = FindSceneInstance(m_target.root.name);
            if (instance == null)
            {
                return null;
            }
            var target = m_target.ResolveIn(instance);
            if (target == null)
            {
                Debug.LogError(
                    $"{k_LogPrefix} target '{m_target.targetName}' " +
                    $"[{string.Join('/', m_target.indices)}] not found under '{instance.name}'",
                    this);
            }
            return target;
        }

        // The child of this plane's parent named `rootName`, or null (with
        // the reason logged as an error).
        private Transform FindSceneInstance(string rootName)
        {
            var parent = transform.parent;
            if (parent == null)
            {
                Debug.LogError($"{k_LogPrefix} no parent to find '{rootName}' under", this);
                return null;
            }
            for (var i = 0; i < parent.childCount; i++)
            {
                var child = parent.GetChild(i);
                if (child.name == rootName)
                {
                    return child;
                }
            }
            Debug.LogError($"{k_LogPrefix} no '{rootName}' beside this plane under '{parent.name}'", this);
            return null;
        }

        // Called when the component is added in the inspector: makes the
        // object a translucent green plane (plane mesh if the filter has
        // none), no shadow casting.
        private void Reset()
        {
            var filter = GetComponent<MeshFilter>();
            if (filter.sharedMesh == null)
            {
                var temp = GameObject.CreatePrimitive(PrimitiveType.Plane);
                filter.sharedMesh = temp.GetComponent<MeshFilter>().sharedMesh;
                DestroyImmediate(temp);
            }
            var meshRenderer = GetComponent<MeshRenderer>();
            var material = new Material(Shader.Find("Sprites/Default"))
            {
                color = new Color(0.2f, 1f, 0.3f, 0.3f),
            };
            meshRenderer.sharedMaterial = material;
            meshRenderer.shadowCastingMode = ShadowCastingMode.Off;
        }
    }
}
