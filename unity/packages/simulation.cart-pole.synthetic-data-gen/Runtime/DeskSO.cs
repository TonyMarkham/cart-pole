using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // One registered desk: the imported FBX as a prefab plus the
    // placement modifiers applied at spawn time - position, rotation
    // (euler degrees) and scale. The model's pivot sits at the
    // user-facing edge of the table top: origin on the top surface at
    // the front edge, the body extending backwards (-Z) and down - so
    // the authored position is where the tabletop edge lands, not the
    // desk's feet.
    [CreateAssetMenu(menuName = MenuManager.DeskMenuName, fileName = MenuManager.DeskFileName)]
    public sealed class DeskSO : ScriptableObject
    {
        public string label;

        public GameObject prefab;

        public Vector3 position;

        public Vector3 localEuler;

        public Vector3 scale = Vector3.one;

        // The prefix of a spawned desk's name; the rest is its label, e.g.
        // "Desk_Classic", so the hierarchy shows which desk is in the scene.
        private const string k_InstanceNamePrefix = "Desk_";

        // Every warning starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[DeskSO]";

        // The warning for a desk with nothing to spawn: {0} is k_LogPrefix,
        // {1} the asset's name.
        private const string k_NoPrefabWarning = "{0} '{1}' has no prefab";

        // The desk placed in the scene: the prefab at `position`, rotated by
        // `localEuler` and scaled by `scale`, all in world space (the desk
        // has no parent). Both modes place a desk through this, so a desk
        // stands in the same place whichever mode chose it. Null, with a
        // warning, when the desk has no prefab.
        public GameObject Spawn()
        {
            if (prefab == null)
            {
                Debug.LogWarning(string.Format(k_NoPrefabWarning, k_LogPrefix, name), this);
                return null;
            }
            var instance = Instantiate(prefab, position, Quaternion.Euler(localEuler));
            instance.transform.localScale = scale;
            instance.name = $"{k_InstanceNamePrefix}{label}";
            return instance;
        }
    }
}
