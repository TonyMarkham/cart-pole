using System;
using System.Collections.Generic;
using Greyhound.Step;
using UnityEngine;

namespace CartPole.Vision
{
    // Applies the rig's marker SOs to the imported assembly: resolves each
    // anchor via its sibling-index path (the same reference style the Joint
    // SOs use), disables the anchor's renderer/colliders (the 1 mm cylinder
    // plip), and creates/updates the marker quads parented to the anchors.
    // The rig is the single source of the marker bindings; the robot is a
    // scene-side reference - a scene instance of the rig's STEP assembly -
    // derived from the rig only as a fallback when it is unset.
    public sealed class MarkerManager : MonoBehaviour
    {
        // The scene instance of the rig's STEP assembly. A scene reference
        // cannot live on the rig SO (an SO holds no scene objects), so the
        // manager - the scene side of the binding - holds it.
        [SerializeField] private GameObject robot;

        // The rig asset whose markers this manager applies. Public so the
        // editor tooling can bind it (it replaces the manager's own marker
        // list, which was public for the same reason).
        public RigSO rig;

        public bool disableAnchorRenderers = true;

        public bool disableAnchorColliders = true;

        public bool applyOnStart;

        private void Start()
        {
            if (applyOnStart)
            {
                Apply();
            }
        }

        [ContextMenu("Apply Markers")]
        public void Apply()
        {
            // Anchors are sibling-index paths into the rig's STEP
            // assembly, so the resolution root is that assembly's scene
            // instance: the robot reference, else derived from the rig
            // when it is unset.
            Transform root = ResolveSceneRoot();
            if (root == null)
            {
                return;
            }
            var applied = 0;
            var classIndex = 0;
            foreach (var marker in rig.Markers)
            {
                if (marker == null || marker.anchor.indices == null ||
                    marker.anchor.indices.Length == 0)
                {
                    Debug.LogWarning($"[MarkerManager] marker '{marker?.label}' has no anchor path");
                    continue;
                }
                var anchor = ResolveAnchor(root, marker.anchor);
                if (anchor == null)
                {
                    Debug.LogWarning(
                        $"[MarkerManager] anchor '{marker.anchor.name}' " +
                        $"[{string.Join('/', marker.anchor.indices)}] not found");
                    continue;
                }

                if (disableAnchorRenderers &&
                    anchor.TryGetComponent<MeshRenderer>(out var renderer))
                {
                    renderer.enabled = false;
                }
                if (disableAnchorColliders)
                {
                    foreach (var collider in anchor.GetComponentsInChildren<Collider>())
                    {
                        collider.enabled = false;
                    }
                }

                // The class id is the marker's position among the rig's
                // non-empty slots - derived, never authored.
                var markerName = $"Marker_{classIndex}_{marker.label}";
                if (marker.prefab == null)
                {
                    Debug.LogWarning($"[MarkerManager] marker '{marker.label}' has no prefab");
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
            Debug.Log($"[MarkerManager] {applied}/{rig.Markers.Count} markers applied");
        }

        // The runtime state payload: the cart's position along the rail
        // in meters and the pole's rotation about the hinge in radians.
        [Serializable]
        private sealed class RigState
        {
            public float cartPosition;
            public float poleRotation;
        }

        // Applies a rig state payload: resolves the cart and pole parts
        // through the rig's sibling paths, then repositions them - the
        // cart translated along the rail by cartPosition meters, the
        // pole rotated by poleRotation radians. The motion convention
        // is the assembly's own: the rail is the cart part's local +X
        // and the hinge is the pole part's local -Z through its own
        // origin (the axes the STEP joint assets author for these
        // parts). Base poses are captured once from the assembled
        // (state = 0) rig.
        public void ApplyState(string json)
        {
            var state = JsonUtility.FromJson<RigState>(json);
            if (state == null)
            {
                Debug.LogWarning("[MarkerManager] unparsable rig state payload");
                return;
            }
            ApplyState(state.cartPosition, state.poleRotation);
        }

        public void ApplyState(float cartPosition, float poleRotation)
        {
            Transform root = ResolveSceneRoot();
            if (root == null)
            {
                return;
            }
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

        // The scene instance the rig's sibling paths index into: the
        // robot reference, else derived from the rig when it is unset.
        // Public: consumers (the capture component) need the same root
        // to locate the marker quads.
        public Transform ResolveSceneRoot()
        {
            if (rig == null)
            {
                Debug.LogWarning("[MarkerManager] no rig assigned - assign the Rig SO in the inspector");
                return null;
            }
            Transform root = robot != null ? robot.transform : ResolveRobot();
            if (root == null)
            {
                Debug.LogWarning(
                    "[MarkerManager] the rig's STEP assembly has no scene instance - " +
                    "drop the assembly instance into the scene");
            }
            return root;
        }

        private sealed class PartPose
        {
            public Transform node;
            public Vector3 localPosition;
            public Quaternion localRotation;
        }

        private PartPose cartPose;
        private PartPose polePose;

        // Base poses from the assembled (state = 0) rig; captured on the
        // first state application or whenever a part went missing.
        private void CaptureBasePoses(Transform root)
        {
            cartPose = CapturePartPose(root, rig.Cart, "cart");
            polePose = CapturePartPose(root, rig.Pole, "pole");
        }

        private PartPose CapturePartPose(Transform root, StepSiblingPath path, string part)
        {
            var node = ResolveAnchor(root, path);
            if (node == null)
            {
                Debug.LogWarning(
                    $"[MarkerManager] {part} '{path.name}' " +
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

        // The fallback resolution: the scene instance of the rig's STEP
        // assembly. In the editor it resolves exactly the way the rig
        // editor's accordion does - the open prefab stage when it stages
        // that asset, else the prefab source of instances in the open
        // scenes; the live stepAsset reference is preferred, the stored
        // path is the fallback for a reference broken by a reimport. A
        // player build has no prefab-source query, so it falls back to
        // matching the assembly's root name.
        private Transform ResolveRobot()
        {
            var stepAsset = rig.StepAsset;
#if UNITY_EDITOR
            var stepPath = stepAsset != null
                ? UnityEditor.AssetDatabase.GetAssetPath(stepAsset)
                : rig.StepAssetPath;
            if (string.IsNullOrEmpty(stepPath))
            {
                // A STEP reimport regenerates the imported objects with
                // fresh file IDs; a stale reference reads as null.
                Debug.LogWarning(
                    "[MarkerManager] the rig has no STEP asset - rebind it in the Rig inspector " +
                    "(a reimport invalidates stale references)");
                return null;
            }
            var stage = UnityEditor.SceneManagement.PrefabStageUtility.GetCurrentPrefabStage();
            if (stage != null && stage.assetPath == stepPath && stage.prefabContentsRoot != null)
            {
                return stage.prefabContentsRoot.transform;
            }
            for (var i = 0; i < UnityEngine.SceneManagement.SceneManager.sceneCount; i++)
            {
                foreach (var sceneRoot in
                    UnityEngine.SceneManagement.SceneManager.GetSceneAt(i).GetRootGameObjects())
                {
                    var found = FindInstanceRoot(sceneRoot.transform, stepPath);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
            Debug.LogWarning($"[MarkerManager] no scene instance of '{stepPath}' found");
            return null;
#else
            var rootName = stepAsset != null
                ? stepAsset.name
                : System.IO.Path.GetFileNameWithoutExtension(rig.StepAssetPath);
            if (string.IsNullOrEmpty(rootName))
            {
                return null;
            }
            for (var i = 0; i < SceneManager.sceneCount; i++)
            {
                foreach (var sceneRoot in SceneManager.GetSceneAt(i).GetRootGameObjects())
                {
                    var found = FindNamedRoot(sceneRoot.transform, rootName);
                    if (found != null)
                    {
                        return found;
                    }
                }
            }
            return null;
#endif
        }

#if UNITY_EDITOR
        private static Transform FindInstanceRoot(Transform node, string stepPath)
        {
            var instanceRoot = UnityEditor.PrefabUtility.GetNearestPrefabInstanceRoot(node.gameObject);
            if (instanceRoot != null &&
                UnityEditor.PrefabUtility.GetPrefabAssetPathOfNearestInstanceRoot(instanceRoot) == stepPath)
            {
                return instanceRoot.transform;
            }
            foreach (Transform child in node)
            {
                var found = FindInstanceRoot(child, stepPath);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }
#else
        private static Transform FindNamedRoot(Transform node, string name)
        {
            if (node.name == name)
            {
                return node;
            }
            foreach (Transform child in node)
            {
                var found = FindNamedRoot(child, name);
                if (found != null)
                {
                    return found;
                }
            }
            return null;
        }
#endif

        private Transform ResolveAnchor(Transform root, StepSiblingPath path)
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
    }
}
