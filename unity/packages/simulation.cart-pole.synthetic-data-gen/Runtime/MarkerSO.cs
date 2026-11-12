using System.Linq;

using Greyhound.Utilities;
using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One ArUco marker definition. All weight lives on this SO: label,
    // texture, size and the anchor identity. The anchor is a
    // SiblingPath - the same rename-robust reference the Joint SOs use -
    // set by dropping the anchor GameObject on the marker inspector.
    // Nothing is stored inside the imported STEP asset; the SOs live in the
    // version-controlled project and the STEP stays disposable. There is no
    // class id field on purpose: a marker's detection class is its position
    // in the MarkerManager's list, applied when the markers are applied -
    // reordering the list renumbers the classes with no asset edits.
    [CreateAssetMenu(menuName = MenuManager.MarkerMenuName, fileName = MenuManager.MarkerFileName)]
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

        // The part the marker sits on; its root is the STEP assembly the
        // indices walk (set by the SiblingPath drawer on drop).
        public SiblingPath anchor = new();

        // The marker prefab (quad + material + geometry), authored at the
        // physical marker size. Instantiated under the resolved anchor.
        public GameObject prefab;

        public float size = 0.05f;

        public Vector3 localOffset = Vector3.zero;

        public Vector3 localEuler = Vector3.zero;

        // Orientation correction for the rendered marker relative to the
        // physical printed marker: flips the texture mapping on the
        // applied quad so the rendered marker matches the physical
        // marker's mounting. Consumed by the MarkerManager at
        // instantiation, the same way localOffset/localEuler are.
        public bool flipVertical;

        public bool flipHorizontal;
    }
}
