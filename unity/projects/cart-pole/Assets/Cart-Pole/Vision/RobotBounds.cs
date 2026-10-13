using UnityEngine;
using UnityEngine.Rendering;

namespace CartPole.Vision
{
    // The robot workspace envelope: a plain cube GameObject. Scale and
    // position it like any object - its renderer bounds are the frustum
    // contract the camera jitter honors. Making it invisible is the
    // renderer's checkbox; captures hide it automatically even when the
    // renderer is on, so it never pollutes a frame.
    [RequireComponent(typeof(MeshFilter), typeof(MeshRenderer))]
    public sealed class RobotBounds : MonoBehaviour
    {
        public Bounds Bounds => GetComponent<Renderer>().bounds;

        // Called when the component is added in the inspector: makes the
        // cube visible as a translucent green block (cube mesh if the
        // filter has none), no shadow casting.
        private void Reset()
        {
            var filter = GetComponent<MeshFilter>();
            if (filter.sharedMesh == null)
            {
                var temp = GameObject.CreatePrimitive(PrimitiveType.Cube);
                filter.sharedMesh = temp.GetComponent<MeshFilter>().sharedMesh;
                DestroyImmediate(temp);
            }
            var meshRenderer = GetComponent<MeshRenderer>();
            var material = new Material(Shader.Find("Sprites/Default"))
            {
                color = new Color(0.2f, 1f, 0.3f, 0.3f),
            };
            meshRenderer.sharedMaterial = material;
            meshRenderer.shadowCastingMode = ShadowCastingMode.Off;
        }
    }
}
