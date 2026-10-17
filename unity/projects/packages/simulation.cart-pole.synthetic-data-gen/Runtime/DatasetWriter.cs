using System;
using System.Collections.Generic;
using System.IO;

using UnityEngine;

namespace Simulation.CartPole.SyntheticDataGen
{
        // The run's file side. One run_<set>_<timestamp> folder per set
        // under the set's output directory - the set's SO name rides in
        // the folder name for record keeping, so the tree shows which
        // set produced which run without opening markers.json. Runs can
        // never mix and no directory preparation exists. The folder is
        // the format's own shape: images/ and labels/ siblings paired
        // by stem, markers.json at the run root - there is no wip~
        // sidecar; the instruments are in-memory steps that leave no
        // files.
    //
    // Labels are write-once: a label file is written a single time
    // with only the gate-confirmed lines, so no rewrite exists to
    // refuse. An empty label file is legitimate - a frame where the
    // gate confirmed nothing is a background frame.
    internal sealed class DatasetWriter
    {
        internal string RunDirectory { get; }

        private bool markersJsonWritten;
        private int frames;

        internal DatasetWriter(string outputDirectory, string setName)
        {
            Directory.CreateDirectory(outputDirectory);
            var stamp = DateTime.Now.ToString("yyyyMMdd_HHmmss");
            RunDirectory = $"{outputDirectory}/run_{setName}_{stamp}";
            for (var suffix = 2; Directory.Exists(RunDirectory); suffix++)
            {
                RunDirectory = $"{outputDirectory}/run_{setName}_{stamp}_{suffix}";
            }
            Directory.CreateDirectory($"{RunDirectory}/images");
            Directory.CreateDirectory($"{RunDirectory}/labels");
        }

        // The class map, written once at the run root from the rig's
        // marker order - the document that makes the labels' class ids
        // interpretable. Class ids are the rig slot order (a marker's
        // position among the rig's non-empty slots); a marker without
        // a valid tag id still consumes its slot but cannot appear
        // here, since nothing can verify it.
        internal void WriteMarkersJson(RigSO rig)
        {
            if (markersJsonWritten)
            {
                return;
            }
            markersJsonWritten = true;
            var list = new MarkerList { markers = new List<MarkerEntry>() };
            var classIndex = 0;
            foreach (var marker in rig.Markers)
            {
                if (marker != null && marker.TagId >= 0)
                {
                    // The detector's plain int id, exported as-is - no
                    // consumer reassembles the id from parts.
                    list.markers.Add(new MarkerEntry
                    {
                        classId = classIndex,
                        label = marker.label,
                        id = marker.TagId,
                    });
                }
                classIndex++;
            }
            File.WriteAllText(
                $"{RunDirectory}/markers.json",
                JsonUtility.ToJson(list, true));
        }

        internal string NextFrameName()
        {
            return $"frame_{frames:D6}";
        }

        // One capture: the greyscale JPEG into images/ and its gated
        // label lines into labels/ - the pairing is the stem, the
        // format's whole mechanism. The label file is always paired
        // with the image, even when empty - absence is never
        // meaningful in this format.
        internal void WriteFrame(string name, byte[] jpeg, List<string> labelLines)
        {
            File.WriteAllBytes($"{RunDirectory}/images/{name}.jpg", jpeg);
            var builder = new System.Text.StringBuilder();
            foreach (var line in labelLines)
            {
                builder.Append(line).Append('\n');
            }
            File.WriteAllText($"{RunDirectory}/labels/{name}.txt", builder.ToString());
            frames++;
        }

        internal int Frames => frames;

        [Serializable]
        private sealed class MarkerEntry
        {
            public int classId;
            public string label;
            public int id;
        }

        [Serializable]
        private sealed class MarkerList
        {
            public List<MarkerEntry> markers;
        }
    }
}
