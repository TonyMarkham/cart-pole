using System.Linq;

using Greyhound.Step;
using UnityEngine;

namespace CartPole.Vision
{
    // One ArUco marker definition. All weight lives on this SO: label,
    // texture, size and the anchor identity. The anchor is a
    // StepSiblingPath - the same rename-robust reference the Joint SOs use -
    // built by dragging the anchor GameObject into the MarkerManager editor.
    // Nothing is stored inside the imported STEP asset; the SOs live in the
    // version-controlled project and the STEP stays disposable. There is no
    // class id field on purpose: a marker's detection class is its position
    // among the rig's markers, applied when the markers are applied -
    // reordering the rig renumbers the classes with no asset edits.
    [CreateAssetMenu(menuName = "Cart-Pole/ArUco Marker", fileName = "Marker")]
    public sealed class MarkerSO : ScriptableObject
    {
        public string label;

        // The ArUco tag id this marker encodes (DICT_4X4_1000), authored
        // as a string so the leading zeros survive ("0376"). Validated
        // against the dictionary type on edit: digits only, four
        // characters, zero-padded, value within the dictionary's range.
        public string arUcoId = "0000";

        // The tag id as OpenCV reports it; -1 when the string is not a
        // valid DICT_4X4_1000 id.
        public int TagId =>
            int.TryParse(arUcoId, out var id) && id >= 0 && id <= 999 ? id : -1;

        private void OnValidate()
        {
            // Validation only - no rewriting. Rewriting mid-typing
            // mangles the input (that is how "0937" became "3070"): the
            // authored string stays exactly as typed, and TagId parses
            // it numerically.
            var id = TagId;
            if (id < 0 || arUcoId.Length != 4 || arUcoId.Any(c => c is < '0' or > '9'))
            {
                Debug.LogWarning(
                    $"[MarkerSO] '{arUcoId}' is not a DICT_4X4_1000 tag id " +
                    "(exactly four digits, 0000-0999)");
            }
        }

        public StepSiblingPath anchor = new();

        // The imported STEP asset the anchor path indexes into - the same
        // resolution basis the Joint SOs get from their joint set's
        // stepAssetPath. Written at creation (marker menu) and at rebind
        // (inspector drop zone); the accordion resolves the live chain
        // through it, no consumer required.
        public string stepAssetPath;

        // The marker prefab (quad + material + geometry), authored at the
        // physical marker size. Instantiated under the resolved anchor.
        public GameObject prefab;

        public float size = 0.05f;

        public Vector3 localOffset = Vector3.zero;

        public Vector3 localEuler = Vector3.zero;
    }
}
