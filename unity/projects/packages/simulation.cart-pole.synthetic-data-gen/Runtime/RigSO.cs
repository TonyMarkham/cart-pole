using System.Collections.Generic;

using Greyhound.Step;
using Unity.Mathematics;
using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    [CreateAssetMenu(menuName = MenuManager.RigMenuName, fileName = MenuManager.RigFileName)]
    public sealed class RigSO : ScriptableObject
    {
        [SerializeField] private MarkerSO[] m_Markers;
        public IReadOnlyList<MarkerSO> Markers => m_Markers;

        [SerializeField] private StepSiblingPath m_Cart = new();
        public StepSiblingPath Cart => m_Cart;
        
        [SerializeField] private StepSiblingPath m_Pole = new();
        public StepSiblingPath Pole => m_Pole;

        // The state draw bounds, in the assembly's own motion units: cart
        // position in meters along the rail, pole rotation in radians
        // about the hinge. Every state the generator draws stays within
        // [x, y]. Defaults are the legacy randomizer's authored ranges -
        // the cart's ±0.45 m is the slider joint's own limit.
        [SerializeField] private float2 m_CartPositionRange = new(-0.45f, 0.45f);
        public float2 CartPositionRange => m_CartPositionRange;

        [SerializeField] private float2 m_PoleRotationRange = new(-Mathf.PI, Mathf.PI);
        public float2 PoleRotationRange => m_PoleRotationRange;

        
        [SerializeField] private GameObject m_StepAsset;
        public GameObject StepAsset => m_StepAsset;
        
        [SerializeField] private string m_StepAssetPath;
        public string StepAssetPath => m_StepAssetPath;
        
        [SerializeField] private bool m_DisableAnchorRenderer = true;
        public bool DisableAnchorRenderer => m_DisableAnchorRenderer;
        
        [SerializeField] private bool m_EnableAnchorCollider = true;
        public bool EnableAnchorCollider => m_EnableAnchorCollider;
        
        [SerializeField] private bool m_EnableOnStart = true;
        public bool EnableOnStart => m_EnableOnStart;

        public void Apply(GameObject assemblyInstance)
        {
            if (assemblyInstance == null)
            {
                Debug.LogWarning("[RigSO] no assembly instance to apply markers to");
                return;
            }
            Transform root = assemblyInstance.transform;
            var applied = 0;
            var classIndex = 0;
            foreach (var marker in Markers)
            {
                if (marker == null || marker.anchor.indices == null ||
                    marker.anchor.indices.Length == 0)
                {
                    Debug.LogWarning($"[RigSO] marker '{marker?.label}' has no anchor path");
                    continue;
                }
                var anchor = ResolveAnchor(root, marker.anchor);
                if (anchor == null)
                {
                    Debug.LogWarning(
                        $"[RigSO] anchor '{marker.anchor.name}' " +
                        $"[{string.Join('/', marker.anchor.indices)}] not found");
                    continue;
                }

                if (m_DisableAnchorRenderer &&
                    anchor.TryGetComponent<MeshRenderer>(out var renderer))
                {
                    renderer.enabled = false;
                }
                if (m_EnableAnchorCollider)
                {
                    foreach (var collider in anchor.GetComponentsInChildren<Collider>())
                    {
                        collider.enabled = false;
                    }
                }

                var markerName = $"Marker_{classIndex}_{marker.label}";
                if (marker.prefab == null)
                {
                    Debug.LogWarning($"[RigSO] marker '{marker.label}' has no prefab");
                    continue;
                }
                var existing = anchor.Find(markerName);
                if (existing != null)
                {
                    DestroyImmediate(existing.gameObject);
                }
                var instance = Instantiate(marker.prefab, anchor);
                instance.name = markerName;
                instance.transform.localPosition = marker.localOffset;
                instance.transform.localRotation = Quaternion.Euler(marker.localEuler);
                // The SO's authored orientation correction: flip the
                // texture mapping on the quad's material. A negative
                // scale mirrors the texture on that axis; the +1 offset
                // keeps the mirrored image inside the [0, 1] UV range.
                // renderer.material (not sharedMaterial): a per-instance
                // material, so the authored flags of one marker never
                // touch another's.
                var markerRenderer = instance.GetComponentInChildren<MeshRenderer>();
                if (markerRenderer != null)
                {
                    var material = markerRenderer.material;
                    material.mainTextureScale = new Vector2(
                        marker.flipHorizontal ? -1f : 1f,
                        marker.flipVertical ? -1f : 1f);
                    material.mainTextureOffset = new Vector2(
                        marker.flipHorizontal ? 1f : 0f,
                        marker.flipVertical ? 1f : 0f);
                }
                applied++;
                classIndex++;
            }
            Debug.Log($"[RigSO] {applied}/{Markers.Count} markers applied");
        }

        // The node a sibling-index path walks to, assembly root first;
        // null where the path no longer resolves.
        public Transform ResolveAnchor(Transform root, StepSiblingPath path)
        {
            if (path.indices == null || path.indices.Length == 0)
            {
                return null;
            }
            var t = root;
            foreach (var index in path.indices)
            {
                if (t == null || index < 0 || index >= t.childCount)
                {
                    return null;
                }
                t = t.GetChild(index);
            }
            return t;
        }

        // The state drive, migrated from the legacy MarkerManager: the
        // cart and pole parts are repositioned to a drawn state. Base
        // poses are captured once from the assembled (state = 0) rig -
        // on the first application, or whenever a part went missing.
        // The motion convention is the assembly's own: the rail is the
        // cart part's local +X, the hinge the pole part's local -Z
        // through its own origin - the ranges m_CartPositionRange and
        // m_PoleRotationRange are authored in these same units.
        public void ApplyState(GameObject assemblyInstance, float cartPosition, float poleRotation)
        {
            if (assemblyInstance == null)
            {
                Debug.LogWarning("[RigSO] no assembly instance to drive");
                return;
            }
            Transform root = assemblyInstance.transform;
            if (cartPose == null || polePose == null ||
                cartPose.node == null || polePose.node == null)
            {
                CaptureBasePoses(root);
            }
            if (cartPose != null)
            {
                cartPose.node.localPosition =
                    cartPose.localPosition + Vector3.right * cartPosition;
            }
            if (polePose != null)
            {
                polePose.node.localRotation = polePose.localRotation *
                    Quaternion.AngleAxis(poleRotation * Mathf.Rad2Deg, Vector3.back);
            }
        }

        private sealed class PartPose
        {
            public Transform node;
            public Vector3 localPosition;
            public Quaternion localRotation;
        }

        private PartPose cartPose;
        private PartPose polePose;

        private void CaptureBasePoses(Transform root)
        {
            cartPose = CapturePartPose(root, m_Cart, "cart");
            polePose = CapturePartPose(root, m_Pole, "pole");
        }

        private PartPose CapturePartPose(Transform root, StepSiblingPath path, string part)
        {
            var node = ResolveAnchor(root, path);
            if (node == null)
            {
                Debug.LogWarning(
                    $"[RigSO] {part} '{path.name}' " +
                    $"[{string.Join('/', path.indices)}] not found");
                return null;
            }
            return new PartPose
            {
                node = node,
                localPosition = node.localPosition,
                localRotation = node.localRotation,
            };
        }
    }
}
