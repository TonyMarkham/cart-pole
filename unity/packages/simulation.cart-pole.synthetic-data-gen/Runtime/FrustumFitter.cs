using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
    // The frustum-compensation dolly: after the random camera
    // transforms, the camera moves along its view axis until the
    // RobotBounds is fully inside the frustum. The movement is the
    // camera's Z adjustment - with the authored pose looking down
    // world +Z and the wander capped at +-5 degrees, the view axis is
    // the Z position for every practical purpose. False means the
    // required compensation exceeded the set's Max Z cap: the caller
    // discards the draw (not captured, does not count toward States)
    // and the camera is put back where the draw left it for the next
    // draw to overwrite.
    internal static class FrustumFitter
    {
        internal static bool Fit(Camera camera, Bounds bounds, float maxZ)
        {
            var transform = camera.transform;
            var start = transform.position;
            var back = -camera.transform.forward;
            if (Fits(camera, bounds))
            {
                return true;
            }
            var distance = 0f;
            var step = 0.005f;
            while (true)
            {
                distance += step;
                step *= 1.5f;
                if (distance > maxZ)
                {
                    transform.position = start;
                    return false;
                }
                transform.position = start + back * distance;
                if (Fits(camera, bounds))
                {
                    return true;
                }
            }
        }

        // True when all eight corners of the bounds project inside the
        // camera's viewport.
        private static bool Fits(Camera camera, Bounds bounds)
        {
            var ext = bounds.extents;
            for (var i = 0; i < 8; i++)
            {
                var corner = bounds.center + new Vector3(
                    (i & 1) == 0 ? -ext.x : ext.x,
                    (i & 2) == 0 ? -ext.y : ext.y,
                    (i & 4) == 0 ? -ext.z : ext.z);
                var viewport = camera.WorldToViewportPoint(corner);
                if (viewport.z <= 0f || viewport.x < 0f || viewport.x > 1f ||
                    viewport.y < 0f || viewport.y > 1f)
                {
                    return false;
                }
            }
            return true;
        }
    }
}
