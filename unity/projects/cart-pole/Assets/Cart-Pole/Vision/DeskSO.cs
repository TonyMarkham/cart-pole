using UnityEngine;

namespace CartPole.Vision
{
    // One registered desk: the imported FBX as a prefab plus the
    // placement modifiers applied at spawn time - position, rotation
    // (euler degrees) and scale. The model's pivot sits at the
    // user-facing edge of the table top: origin on the top surface at
    // the front edge, the body extending backwards (-Z) and down - so
    // the authored position is where the tabletop edge lands, not the
    // desk's feet.
    [CreateAssetMenu(menuName = "Cart-Pole/Desk", fileName = "Desk")]
    public sealed class DeskSO : ScriptableObject
    {
        public string label;

        public GameObject prefab;

        public Vector3 position;

        public Vector3 localEuler;

        public Vector3 scale = Vector3.one;
    }
}
