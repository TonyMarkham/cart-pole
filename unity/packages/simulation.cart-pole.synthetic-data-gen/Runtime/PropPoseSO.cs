using UnityEngine;

using SiblingPath = Greyhound.Utilities.SiblingPath;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One prop placement: a prefab at a fixed local pose relative to a
    // part of the robot. The part is a sibling-index path into the robot's
    // STEP assembly - the utilities package's SiblingPath, whose property
    // drawer builds it from the part - and the prop is parented there,
    // so it follows the part wherever a pose puts it: a prop placed on
    // the cart rides with the cart.
    [CreateAssetMenu(menuName = MenuManager.PropPoseMenuName, fileName = MenuManager.PropPoseFileName)]
    public sealed class PropPoseSO : ScriptableObject
    {
        [SerializeField] private SiblingPath m_Parent = new();
        public SiblingPath Parent => m_Parent;

        [SerializeField] private GameObject m_Prefab;
        public GameObject Prefab => m_Prefab;

        // The prop's pose in the parent part's own frame: position in
        // meters (greyhound bakes the import scale into the vertices,
        // so every part transform is unit scale), rotation as Euler
        // angles in degrees.
        [SerializeField] private Vector3 m_LocalPosition;
        public Vector3 LocalPosition => m_LocalPosition;

        [SerializeField] private Vector3 m_LocalEuler;
        public Vector3 LocalEuler => m_LocalEuler;

        // Instantiates the prop under the part the path names inside the
        // assembly instance, at the authored local pose. Null (and a
        // warning) when there is no prefab or the path no longer
        // resolves - the frame is then captured without this prop.
        public GameObject Place(GameObject assemblyInstance)
        {
            if (m_Prefab == null)
            {
                Debug.LogWarning($"[PropPoseSO] '{name}' has no prefab", this);
                return null;
            }
            var parent = Resolve(assemblyInstance.transform);
            if (parent == null)
            {
                var indices = m_Parent.indices ?? new int[0];
                Debug.LogWarning(
                    $"[PropPoseSO] '{name}': part '{m_Parent.targetName}' " +
                    $"[{string.Join("/", indices)}] not found under '{assemblyInstance.name}'",
                    this);
                return null;
            }
            var instance = Instantiate(m_Prefab, parent);
            instance.transform.localPosition = m_LocalPosition;
            instance.transform.localRotation = Quaternion.Euler(m_LocalEuler);
            return instance;
        }

        // Walks the path's child indices down from the instance root;
        // SiblingPath.SELF names the node it is at, so a path of [SELF]
        // is the instance root itself.
        private Transform Resolve(Transform root)
        {
            var indices = m_Parent.indices;
            if (indices == null || indices.Length == 0)
            {
                return null;
            }
            var node = root;
            foreach (var index in indices)
            {
                if (index == SiblingPath.SELF)
                {
                    continue;
                }
                if (index < 0 || index >= node.childCount)
                {
                    return null;
                }
                node = node.GetChild(index);
            }
            return node;
        }
    }
}
