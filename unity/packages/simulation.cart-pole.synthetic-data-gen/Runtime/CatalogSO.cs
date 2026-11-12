using System.Collections.Generic;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // Every desk, skybox (HDRI material) and prop pose a generation set can
    // name, so a player can turn the names in a GenerationSet message into
    // assets. A name is the asset's own name (its file name without the
    // extension): "Desk (Writing)", "pav_studio_03_4k", "Prop Pose - Idler",
    // the names the JSON sets in __synthetic__/generator/sets/ use.
    //
    // It is also what gets those assets into a player build: Unity builds in
    // only what the build's scenes reference, so the scene's
    // SyntheticDataGenerator references this catalog, and the catalog
    // references every desk, skybox and prop (and through them their
    // prefabs, meshes and textures).
    //
    // A name must be unique within its list; OnValidate warns about an empty
    // entry or a repeated name, and a lookup of a name that is not listed
    // fails rather than picking something else.
    [CreateAssetMenu(menuName = MenuManager.CatalogMenuName, fileName = MenuManager.CatalogFileName)]
    public sealed class CatalogSO : ScriptableObject
    {
        // Every warning starts with this, so the Console shows who wrote it.
        private const string k_LogPrefix = "[CatalogSO]";

        // What each list holds, as its warnings name it ("desk entry 2 is
        // empty").
        private const string k_DeskKind = "desk";
        private const string k_SkyboxKind = "skybox";
        private const string k_PropPoseKind = "prop pose";

        [SerializeField] private List<DeskSO> m_Desks = new();
        public IReadOnlyList<DeskSO> Desks => m_Desks;

        [SerializeField] private List<Material> m_Skyboxes = new();
        public IReadOnlyList<Material> Skyboxes => m_Skyboxes;

        [SerializeField] private List<PropPoseSO> m_PropPoses = new();
        public IReadOnlyList<PropPoseSO> PropPoses => m_PropPoses;

        public bool TryGetDesk(string assetName, out DeskSO desk)
        {
            return TryFind(m_Desks, assetName, out desk);
        }

        public bool TryGetSkybox(string assetName, out Material skybox)
        {
            return TryFind(m_Skyboxes, assetName, out skybox);
        }

        public bool TryGetPropPose(string assetName, out PropPoseSO propPose)
        {
            return TryFind(m_PropPoses, assetName, out propPose);
        }

        private void OnValidate()
        {
            WarnAboutEntries(m_Desks, k_DeskKind);
            WarnAboutEntries(m_Skyboxes, k_SkyboxKind);
            WarnAboutEntries(m_PropPoses, k_PropPoseKind);
        }

        private static bool TryFind<T>(List<T> entries, string assetName, out T found)
            where T : Object
        {
            foreach (var entry in entries)
            {
                if (entry != null && entry.name == assetName)
                {
                    found = entry;
                    return true;
                }
            }
            found = null;
            return false;
        }

        private void WarnAboutEntries<T>(List<T> entries, string kind)
            where T : Object
        {
            var names = new HashSet<string>();
            for (var i = 0; i < entries.Count; i++)
            {
                if (entries[i] == null)
                {
                    Debug.LogWarning($"{k_LogPrefix} '{name}': {kind} entry {i} is empty", this);
                }
                else if (!names.Add(entries[i].name))
                {
                    Debug.LogWarning($"{k_LogPrefix} '{name}': two {kind} entries are named '{entries[i].name}'", this);
                }
            }
        }
    }
}
