using System.Collections.Generic;

using Greyhound.Step;
using UnityEngine;

namespace CartPole.Vision
{
    // One rig binding: the four Marker SOs of the rig plus the
    // StepSiblingPath identities of the cart and pole parts of the
    // imported STEP assembly - the same rename-robust reference style
    // the Marker SOs use for their anchors. All weight lives on this SO;
    // nothing is stored inside the imported STEP asset. An SO cannot
    // hold a scene reference, so the scene-side MarkerManager consumes
    // this asset rather than being referenced by it. The bindings are
    // written by dragging into this asset's inspector drop zones.
    [CreateAssetMenu(menuName = "Cart-Pole/Rig", fileName = "Rig")]
    public sealed class RigSO : ScriptableObject
    {
        // The four markers of the rig (End, Left, Pivot, Right).
        [SerializeField] private MarkerSO[] markers = new MarkerSO[4];

        // Read access for the scene-side consumer (the MarkerManager);
        // the bindings themselves are written through the inspector.
        public IReadOnlyList<MarkerSO> Markers => markers;

        [SerializeField] private StepSiblingPath cart = new();

        [SerializeField] private StepSiblingPath pole = new();

        // The imported STEP assembly asset the cart and pole paths index
        // into - the direct analog of MarkerSO.stepAssetPath, held as a
        // reference rather than a path string. Written by the inspector
        // drop zones; the accordions resolve the live chains through it.
        [SerializeField] private GameObject stepAsset;

        // The path of the STEP assembly, written alongside the reference
        // at rebind time. A STEP reimport regenerates the imported
        // objects with fresh file IDs and breaks the reference - the
        // path string survives, so resolution falls back to it.
        [SerializeField] private string stepAssetPath;

        // Read access for the scene-side consumer; the bindings
        // themselves are written through the inspector.
        public GameObject StepAsset => stepAsset;

        public string StepAssetPath => stepAssetPath;

        public StepSiblingPath Cart => cart;

        public StepSiblingPath Pole => pole;
    }
}
