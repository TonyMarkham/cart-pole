using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // Per-draw environment: desk spawn, skybox and light application
    // from the set's authored pools. Every range is absolute - the
    // drawn values ARE the world values (light euler (pitch, yaw, 0),
    // skybox material _Rotation), not offsets around the scene's
    // authored poses. A randomized-off element leaves the scene as the
    // previous state left it, except the desk: a desk-free draw
    // removes the desk, matching the legacy behavior.
    internal sealed class EnvironmentRandomizer
    {
        private readonly GenerationSetSO set;
        private readonly System.Random random;
        private readonly Light light;
        private GameObject deskInstance;
        private bool warnedNoDesks;
        private bool warnedNoSkyboxes;
        private bool warnedNoLight;

        internal EnvironmentRandomizer(
            GenerationSetSO set, System.Random random, Light light)
        {
            this.set = set;
            this.random = random;
            this.light = light;
        }

        internal void Apply()
        {
            ApplyDesk();
            ApplySkybox();
            ApplyLight();
        }

        // The desk for this draw: a random entry from the pool, spawned
        // with the SO's placement modifiers (the pivot sits at the
        // tabletop's front edge), replacing the previous instance.
        // The set's run is over: the desk goes with it. The next set's
        // randomizer knows nothing of this instance - without this,
        // every set boundary would orphan one active desk into the
        // scene, and every later set would render with all previous
        // sets' desks stacked in the background.
        internal void Shutdown()
        {
            RemoveDesk();
        }

        private void ApplyDesk()
        {
            if (!set.RandomizeDesks)
            {
                RemoveDesk();
                return;
            }
            if (set.Desks == null || set.Desks.Count == 0)
            {
                if (!warnedNoDesks)
                {
                    warnedNoDesks = true;
                    Debug.LogWarning(
                        "[EnvironmentRandomizer] desk randomization is on " +
                        "but the set's desk pool is empty");
                }
                return;
            }
            var desk = set.Desks[random.Next(set.Desks.Count)];
            if (desk == null || desk.prefab == null)
            {
                Debug.LogWarning(
                    "[EnvironmentRandomizer] a desk in the set's pool has no prefab");
                return;
            }
            RemoveDesk();
            deskInstance = Object.Instantiate(
                desk.prefab, desk.position, Quaternion.Euler(desk.localEuler));
            deskInstance.transform.localScale = desk.scale;
            deskInstance.name = $"Desk_{desk.label}";
        }

        // The spawned desk, if any, removed. Deactivated first, then
        // Destroy: the run's awaits can execute several state chunks
        // between player-loop frames (an unfocused editor throttles
        // the loop but still pumps the synchronization context), so a
        // frame can render before the deferred destruction takes
        // effect - and would show the previous desk alongside the new
        // one. Deactivation is immediate, so no frame ever renders
        // the corpse; the actual destruction follows at frame end,
        // as the runtime API intends.
        private void RemoveDesk()
        {
            if (deskInstance == null)
            {
                return;
            }
            deskInstance.SetActive(false);
            Object.Destroy(deskInstance);
            deskInstance = null;
        }

        // The skybox for this draw: a random HDRI material from the
        // pool with an absolute yaw about Unity's Y (the material's
        // _Rotation) - even the same HDRI presents a different view per
        // state. Applied through RenderSettings so the capture camera
        // sees it.
        private void ApplySkybox()
        {
            if (!set.RandomizeSkyboxes)
            {
                return;
            }
            if (set.Skyboxes == null || set.Skyboxes.Count == 0)
            {
                if (!warnedNoSkyboxes)
                {
                    warnedNoSkyboxes = true;
                    Debug.LogWarning(
                        "[EnvironmentRandomizer] skybox randomization is on " +
                        "but the set's skybox pool is empty");
                }
                return;
            }
            var skybox = set.Skyboxes[random.Next(set.Skyboxes.Count)];
            if (skybox == null)
            {
                return;
            }
            var yaw = Draw(set.SkyboxYawRange);
            skybox.SetFloat("_Rotation", yaw);
            RenderSettings.skybox = skybox;
        }

        // The light for this draw: absolute intensity and absolute
        // world euler (pitch, yaw, 0) - the set's ranges are the world
        // values, not offsets around the authored sun.
        private void ApplyLight()
        {
            if (!set.RandomizeLightIntensity && !set.RandomizeLightYaw &&
                !set.RandomizeLightPitch)
            {
                return;
            }
            if (light == null)
            {
                if (!warnedNoLight)
                {
                    warnedNoLight = true;
                    Debug.LogWarning(
                        "[EnvironmentRandomizer] light randomization is on " +
                        "but no directional light is assigned");
                }
                return;
            }
            light.intensity = Draw(set.LightIntensityRange);
            light.transform.rotation = Quaternion.Euler(
                Draw(set.LightPitchRange), Draw(set.LightYawRange), 0f);
        }

        private float Draw(float2 range)
        {
            return range.x + (float)random.NextDouble() * (range.y - range.x);
        }
    }
}
