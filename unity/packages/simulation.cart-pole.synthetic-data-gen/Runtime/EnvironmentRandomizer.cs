using System.Collections.Generic;

using Unity.Mathematics;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // Per-draw environment: desk spawn, skybox and light application
    // from the set's pools. Every range is absolute - the drawn values
    // ARE the world values (light euler (pitch, yaw, 0), skybox
    // _Rotation), not offsets around the scene's authored poses. A range
    // the set leaves out draws nothing and leaves that value as the scene
    // has it; an empty desk list means no desk.
    //
    // The draw order - desk, skybox, skybox yaw, light intensity, pitch,
    // yaw - is part of the random sequence a seed replays.
    //
    // The scene is put back at Shutdown (light, skybox), so the next set
    // starts from the scene, not from this set's last draw. A skybox is
    // turned on a copy of its material, never the material asset itself,
    // so no run changes a project asset.
    internal sealed class EnvironmentRandomizer
    {
        // The skybox shader property that turns the HDRI about the world's
        // up axis, in degrees (Unity's Skybox/Panoramic and Skybox/Cubemap
        // shaders).
        private const string k_SkyboxRotationProperty = "_Rotation";

        private readonly ResolvedGenerationSet set;
        private readonly System.Random random;
        private readonly Light light;
        private readonly Material sceneSkybox;
        private readonly float sceneLightIntensity;
        private readonly Quaternion sceneLightRotation;
        private readonly Dictionary<Material, Material> skyboxCopies = new();
        private GameObject deskInstance;
        private bool warnedNoLight;

        internal EnvironmentRandomizer(
            ResolvedGenerationSet set, System.Random random, Light light)
        {
            this.set = set;
            this.random = random;
            this.light = light;
            sceneSkybox = RenderSettings.skybox;
            if (light != null)
            {
                sceneLightIntensity = light.intensity;
                sceneLightRotation = light.transform.rotation;
            }
        }

        internal void Apply()
        {
            ApplyDesk();
            ApplySkybox();
            ApplyLight();
        }

        // The set's run is over: its desk goes, the scene's skybox and
        // light come back, and the skybox copies are freed. Without this,
        // every set boundary would orphan one active desk into the scene,
        // and the next set would start from this set's last draw.
        internal void Shutdown()
        {
            RemoveDesk();
            RenderSettings.skybox = sceneSkybox;
            foreach (var copy in skyboxCopies.Values)
            {
                Object.Destroy(copy);
            }
            skyboxCopies.Clear();
            if (light != null)
            {
                light.intensity = sceneLightIntensity;
                light.transform.rotation = sceneLightRotation;
            }
        }

        // The desk for this draw: a random entry from the pool, spawned
        // with the DeskSO's placement modifiers (the pivot sits at the
        // tabletop's front edge), replacing the previous instance.
        private void ApplyDesk()
        {
            if (set.Desks.Count == 0)
            {
                RemoveDesk();
                return;
            }
            var desk = set.Desks[random.Next(set.Desks.Count)];
            RemoveDesk();
            deskInstance = desk.Spawn();
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

        // The skybox for this draw: a random HDRI from the pool with an
        // absolute yaw about Unity's Y (0 when the set has no skybox_yaw) -
        // even the same HDRI presents a different view per state. Applied
        // through RenderSettings so the capture camera sees it.
        private void ApplySkybox()
        {
            if (set.Skyboxes.Count == 0)
            {
                return;
            }
            var skybox = set.Skyboxes[random.Next(set.Skyboxes.Count)];
            if (!skyboxCopies.TryGetValue(skybox, out var copy))
            {
                copy = new Material(skybox);
                skyboxCopies.Add(skybox, copy);
            }
            var yaw = set.SkyboxYaw.HasValue ? Draw(set.SkyboxYaw.Value) : 0f;
            copy.SetFloat(k_SkyboxRotationProperty, yaw);
            RenderSettings.skybox = copy;
        }

        // The light for this draw: absolute intensity and absolute world
        // euler (pitch, yaw, 0). Each value the set has no range for keeps
        // the scene's: its intensity, or its pitch or yaw.
        private void ApplyLight()
        {
            if (!set.LightIntensity.HasValue && !set.LightPitch.HasValue && !set.LightYaw.HasValue)
            {
                return;
            }
            if (light == null)
            {
                if (!warnedNoLight)
                {
                    warnedNoLight = true;
                    Debug.LogWarning(
                        "[EnvironmentRandomizer] the set draws the light " +
                        "but no directional light is assigned");
                }
                return;
            }
            if (set.LightIntensity.HasValue)
            {
                light.intensity = Draw(set.LightIntensity.Value);
            }
            var sceneEuler = sceneLightRotation.eulerAngles;
            var pitch = set.LightPitch.HasValue ? Draw(set.LightPitch.Value) : sceneEuler.x;
            var yaw = set.LightYaw.HasValue ? Draw(set.LightYaw.Value) : sceneEuler.y;
            light.transform.rotation = Quaternion.Euler(pitch, yaw, 0f);
        }

        private float Draw(float2 range)
        {
            return range.x + (float)random.NextDouble() * (range.y - range.x);
        }
    }
}
