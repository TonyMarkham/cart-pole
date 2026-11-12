using System.Collections.Generic;
using Greyhound.MuJoCo;
using Simulation.CartPole.SyntheticDataGen;
using UnityEngine;

namespace CartPole.Vision
{
    // Owns the robot's ArUco markers and puts them on the imported
    // assembly: resolves each marker's anchor (a SiblingPath), disables the
    // anchor's renderer/colliders (the 1 mm cylinder plip), and creates the
    // marker quad as a child of the anchor, so it moves with the part.
    //
    // A marker's detection class is its index in this list: [idler, motor,
    // pivot, tip] gives idler class 0 ... tip class 3, so reordering the
    // list renumbers the classes with no asset edits. This is the single
    // source of truth for classes: what TryApply placed is handed out as
    // Placed (class id, marker, quad), and the labels, the verification
    // gate, markers.json and data.yaml's names all read it from there.
    //
    // So that an index is always a class, there are no gaps: every entry
    // must be usable (a marker, a valid tag, an anchor that resolves, a
    // prefab). One that is not stops TryApply with an error naming it, and
    // nothing is placed - never a skipped entry that would shift the
    // classes after it.
    //
    // Each anchor is found by walking its sibling indices
    // (SiblingPath.ResolveIn) down from the robot's instance in the scene,
    // which the caller passes to TryApply; nothing is searched for by name.
    // Start and the Apply Markers menu take that instance from the
    // StateServer on this GameObject (StateServer.instance), the scene
    // reference everything on the robot starts from.
    public sealed class MarkerManager : MonoBehaviour
    {
        // Every log line starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[MarkerManager]";

        // A placed quad's name: this prefix, the class id and the label,
        // e.g. "Marker_2_pivot". For reading the hierarchy only; nothing
        // parses it.
        private const string k_QuadNamePrefix = "Marker_";

        // The texture mapping per axis (mainTextureScale, mainTextureOffset):
        // unflipped is scale 1, offset 0; flipped is scale -1, which mirrors
        // the texture on that axis, with offset 1, which moves the mirrored
        // image back inside the [0, 1] UV range.
        private const float k_UnflippedScale = 1f;
        private const float k_UnflippedOffset = 0f;
        private const float k_FlippedScale = -1f;
        private const float k_FlippedOffset = 1f;

        [SerializeField] private List<MarkerSO> markers = new();
        public IReadOnlyList<MarkerSO> Markers => markers;

        public bool disableAnchorRenderers = true;

        public bool disableAnchorColliders = true;

        public bool applyOnStart;

        private readonly List<PlacedMarker> m_Placed = new();

        // The quads the last successful TryApply placed, in class order (index
        // i holds class i); empty before that, or after a TryApply that
        // failed.
        public IReadOnlyList<PlacedMarker> Placed => m_Placed;

        private void Start()
        {
            if (applyOnStart)
            {
                TryApply(StateServerInstance());
            }
        }

        [ContextMenu("Apply Markers")]
        private void ApplyFromMenu()
        {
            TryApply(StateServerInstance());
        }

        // The robot's instance from the StateServer on this GameObject, or
        // null (logged) when there is no StateServer.
        private Transform StateServerInstance()
        {
            var server = GetComponent<StateServer>();
            if (server == null)
            {
                Debug.LogError($"{k_LogPrefix} no StateServer on this GameObject to take the robot's instance from", this);
                return null;
            }
            return server.instance;
        }

        // Places every marker on `instance`, the robot in the scene, or
        // none: false (each problem logged as an error) when there is no
        // instance or any entry is not usable.
        public bool TryApply(Transform instance)
        {
            if (instance == null)
            {
                m_Placed.Clear();
                Debug.LogError($"{k_LogPrefix} no robot instance to place the markers on", this);
                return false;
            }
            m_Placed.Clear();
            var anchors = new List<Transform>();
            var hasProblem = false;
            for (var classId = 0; classId < markers.Count; classId++)
            {
                var anchor = ResolveUsableAnchor(classId, instance);
                hasProblem |= anchor == null;
                anchors.Add(anchor);
            }
            if (hasProblem)
            {
                Debug.LogError($"{k_LogPrefix} no markers applied: fix the entries above", this);
                return false;
            }

            for (var classId = 0; classId < markers.Count; classId++)
            {
                m_Placed.Add(Place(classId, markers[classId], anchors[classId]));
            }
            Debug.Log($"{k_LogPrefix} {m_Placed.Count} markers applied", this);
            return true;
        }

        // The anchor entry `classId` hangs on, or null (with the reason
        // logged as an error) when the entry cannot be placed.
        private Transform ResolveUsableAnchor(int classId, Transform instance)
        {
            var marker = markers[classId];
            if (marker == null)
            {
                Debug.LogError($"{k_LogPrefix} entry {classId} is empty", this);
                return null;
            }
            if (marker.TagId < 0)
            {
                Debug.LogError(
                    $"{k_LogPrefix} entry {classId} '{marker.label}': '{marker.arUcoId}' is not a DICT_4X4_1000 tag id",
                    this);
                return null;
            }
            if (marker.prefab == null)
            {
                Debug.LogError($"{k_LogPrefix} entry {classId} '{marker.label}' has no prefab", this);
                return null;
            }
            if (marker.anchor.indices == null || marker.anchor.indices.Length == 0)
            {
                Debug.LogError($"{k_LogPrefix} entry {classId} '{marker.label}' has no anchor path", this);
                return null;
            }
            var anchor = marker.anchor.ResolveIn(instance);
            if (anchor == null)
            {
                Debug.LogError(
                    $"{k_LogPrefix} entry {classId} '{marker.label}': anchor '{marker.anchor.targetName}' " +
                    $"[{string.Join('/', marker.anchor.indices)}] not found",
                    this);
            }
            return anchor;
        }

        private PlacedMarker Place(int classId, MarkerSO marker, Transform anchor)
        {
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

            // The name is for reading the hierarchy only: nothing parses it.
            var markerName = $"{k_QuadNamePrefix}{classId}_{marker.label}";
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
                    marker.flipHorizontal ? k_FlippedScale : k_UnflippedScale,
                    marker.flipVertical ? k_FlippedScale : k_UnflippedScale);
                material.mainTextureOffset = new Vector2(
                    marker.flipHorizontal ? k_FlippedOffset : k_UnflippedOffset,
                    marker.flipVertical ? k_FlippedOffset : k_UnflippedOffset);
            }
            return new PlacedMarker(classId, marker, instance.transform);
        }
    }
}
