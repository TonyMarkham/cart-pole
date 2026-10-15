using System;

namespace CartPole.Vision
{
    // One frame's verification accounting, the shape
    // verification_stats.json carries (the field names the retired
    // Python checker's artifact used, kept so existing report readers
    // keep working).
    [Serializable]
    public sealed class ArUcoVerifierFrameResult
    {
        public string frame;
        public int confirmed;
        public int dropped;
        public string droppedMarkers;
    }
}
