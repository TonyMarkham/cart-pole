using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One marker quad MarkerManager.TryApply placed: the detection class it
    // stands for, the marker it was made from, and the quad itself. The
    // class id is the marker's index in MarkerManager's list - the single
    // source of truth for classes: the labels, the verification gate,
    // markers.json and data.yaml's names all read it from here, never from
    // the quad's name.
    public sealed class PlacedMarker
    {
        public PlacedMarker(int classId, MarkerSO marker, Transform quad)
        {
            ClassId = classId;
            Marker = marker;
            Quad = quad;
        }

        public int ClassId { get; }

        // Its label (data.yaml's name for the class) and its ArUco tag.
        public MarkerSO Marker { get; }

        public Transform Quad { get; }
    }
}
