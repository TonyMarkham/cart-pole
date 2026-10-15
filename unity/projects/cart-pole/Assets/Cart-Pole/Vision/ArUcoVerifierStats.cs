using System;

namespace CartPole.Vision
{
    // The run's totals, written as verification_stats.json beside the
    // dataset's frames. WriteRefused is a property, so JsonUtility
    // leaves it out of the file: the artifact's schema stays the
    // retired checker's.
    [Serializable]
    public sealed class ArUcoVerifierStats
    {
        public string dataset;
        public int framesScanned;
        public int labelsConfirmed;
        public int labelsDropped;
        public ArUcoVerifierFrameResult[] frames;

        // True when the never-wipe rule refused the rewrite: zero
        // confirmations on a dataset with labels - the wholesale-failure
        // signature the motivating 0/80 run produced - keeps every
        // label as written.
        public bool WriteRefused { get; set; }
    }
}
