using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

using UnityEngine;

namespace CartPole.Vision
{
    // The dataset verify step - aruco_check.py's in-process replacement
    // (the Python checker retired per greyhound vision.md V-D4/V-D7):
    // which images exist, the comparison, the label accounting, the
    // rewrite policy - all project-side, because this is dataset
    // semantics: this repo's capture layout, label policy, and rig
    // mapping. The package carries only the mechanism (Greyhound.
    // Vision.Ghcv, the native detection bridge, and the OpenCV payload
    // provisioner).
    //
    // The dataset contract: every main frame (frame_NNNNNN.jpg) has a
    // YOLO label file beside it and, per marker, a retained close-up
    // crop (frame_NNNNNN_8k_marker_TTTT.jpg) the capture cut from the
    // 8K render, kept in the dataset's wip~ sidecar directory (the
    // Unity- and git-ignored folder; corpora captured before that
    // layout keep the crops at the dataset root). A label line
    // survives only when its class's crop decodes to exactly that
    // class's tag - the crop is one marker's close-up, so any other
    // result (nothing, several markers, a different id) is an
    // unverifiable box; detection absence is the visibility signal
    // that keeps bad labels out of the dataset.
    //
    // Basis variants: a caller may hand in preConfirmed (stem ->
    // confirmed class ids) for crops it detected in memory and did
    // not retain (the capture's GenerateDebugImages=off path); for
    // those frames a line survives when its class sits in the set -
    // no crop file is consulted. Frames without a preConfirmed
    // entry verify against retained crop files exactly as before.
    // The never-wipe accounting is global across both bases.
    //
    // Basis decision (2026-10-14): the frame-level trial - detecting
    // on the 1440x1080 image only - confirmed 66/80 against the crop
    // basis' 80/80 and was reverted; the 8K render and the crops stay.
    //
    // The never-wipe rule - the motivating failure of the Python
    // checker, which rewrote every label to empty on a 0/80 run: a run
    // with zero confirmations refuses the rewrite and keeps the labels
    // as written. Only confirmed lines are ever written.
    public static class ArUcoVerifier
    {
        // Verifies one dataset directory and (unless refused) rewrites
        // its label files to the confirmed lines; returns the
        // accounting. The class-to-tag mapping comes from the caller
        // (the capture resolves it from the rig's Marker SOs) - no
        // markers.json inter-lingua: the retired checker's file
        // contract died with it. preConfirmed carries the in-memory
        // basis for non-retained crops (see the header comment).
        // writeStats=false (the capture's GenerateDebugDocs=off)
        // skips the stats file: the accounting lives in the return
        // value and the log only - the run carries no file-level
        // verification receipt.
        public static ArUcoVerifierStats Verify(
            string datasetDirectory, List<ArUcoVerifierEntry> markers,
            Dictionary<string, HashSet<int>> preConfirmed = null,
            bool writeStats = true)
        {
            var stats = new ArUcoVerifierStats
            {
                dataset = datasetDirectory,
                frames = Array.Empty<ArUcoVerifierFrameResult>(),
            };
            if (!Directory.Exists(datasetDirectory))
            {
                Debug.LogWarning($"[ArUcoVerifier] no dataset directory {datasetDirectory}");
                return stats;
            }
            var tags = new Dictionary<int, int>();
            var labels = new Dictionary<int, string>();
            if (markers != null)
            {
                foreach (var marker in markers)
                {
                    if (marker.tagId < 0)
                    {
                        continue;   // no tag - no crop can verify this class
                    }
                    tags[marker.classId] = marker.tagId;
                    labels[marker.classId] = marker.label;
                }
            }
            var wipDirectory = $"{datasetDirectory}/wip~";
            var frames = new List<ArUcoVerifierFrameResult>();
            var rewrites = new List<Rewrite>();
            var files = Directory.GetFiles(datasetDirectory, "frame_*.jpg");
            Array.Sort(files, StringComparer.Ordinal);
            foreach (string jpg in files)
            {
                string stem = Path.GetFileNameWithoutExtension(jpg);
                if (stem.Contains("_marker_")
                    || stem.EndsWith("_8k", StringComparison.Ordinal))
                {
                    continue;   // the 8K renders and the crops are not main frames
                }
                string txtPath = $"{datasetDirectory}/{stem}.txt";
                if (!File.Exists(txtPath))
                {
                    continue;
                }
                var kept = new List<string>();
                var droppedNames = new List<string>();
                foreach (string line in File.ReadAllLines(txtPath))
                {
                    var parts = line.Split(' ');
                    if (parts.Length != 5 || !int.TryParse(parts[0], out var classId))
                    {
                        continue;   // malformed - the same silent skip the checker had
                    }
                    bool confirmed = false;
                    if (tags.TryGetValue(classId, out int expected))
                    {
                        if (preConfirmed != null &&
                            preConfirmed.TryGetValue(stem, out var set) &&
                            set.Contains(classId))
                        {
                            // In-memory basis: the caller detected this
                            // frame's crops at capture time and did not
                            // retain them.
                            confirmed = true;
                        }
                        else
                        {
                            // File basis: crops live in the wip~
                            // sidecar directory; corpora captured
                            // before that layout keep them at the
                            // dataset root.
                            string cropName = $"{stem}_8k_marker_{expected:D4}.jpg";
                            string cropPath = $"{wipDirectory}/{cropName}";
                            if (!File.Exists(cropPath))
                            {
                                cropPath = $"{datasetDirectory}/{cropName}";
                            }
                            confirmed = File.Exists(cropPath) &&
                                CropConfirms(cropPath, expected);
                        }
                    }
                    if (confirmed)
                    {
                        kept.Add(line);
                        stats.labelsConfirmed++;
                    }
                    else
                    {
                        stats.labelsDropped++;
                        droppedNames.Add(labels.TryGetValue(classId, out string label)
                            ? label
                            : "?");
                    }
                }
                var droppedText = droppedNames.Count > 0
                    ? string.Join(", ", droppedNames)
                    : "none";
                frames.Add(new ArUcoVerifierFrameResult
                {
                    frame = stem,
                    confirmed = kept.Count,
                    dropped = droppedNames.Count,
                    droppedMarkers = droppedText,
                });
                Debug.Log($"[ArUcoVerifier] {stem}.jpg: "
                    + $"{kept.Count} confirmed, {droppedNames.Count} dropped "
                    + $"[{droppedText}]");
                rewrites.Add(new Rewrite { path = txtPath, kept = kept });
            }
            stats.frames = frames.ToArray();
            stats.framesScanned = frames.Count;
            if (stats.labelsConfirmed == 0 && stats.labelsDropped > 0)
            {
                // The never-wipe rule: zero confirmations on a dataset
                // with labels is the wholesale-failure signature (the
                // 0/80 run) - keep every label as written.
                stats.WriteRefused = true;
                Debug.LogWarning(
                    "[ArUcoVerifier] zero confirmations - refusing the label "
                    + "rewrite, labels kept as written");
            }
            else
            {
                foreach (var rewrite in rewrites)
                {
                    var builder = new StringBuilder();
                    foreach (string line in rewrite.kept)
                    {
                        builder.Append(line).Append('\n');
                    }
                    File.WriteAllText(rewrite.path, builder.ToString());
                }
            }
            if (writeStats)
            {
                Directory.CreateDirectory(wipDirectory);
                File.WriteAllText(
                    $"{wipDirectory}/verification_stats.json",
                    JsonUtility.ToJson(stats, true) + "\n");
            }
            return stats;
        }

        // A crop confirms its marker when the detector reads exactly
        // that tag from it: the crop frames one marker, so several ids
        // or a different single id are both an unverifiable box. The
        // retired checker's scale-retry (1x/2x/4x nearest-neighbour) is
        // not ported - the validated corpus run is single-pass at
        // native resolution (80/80 crops through this shim).
        private static bool CropConfirms(string cropPath, int expected)
        {
            return CropConfirms(
                File.ReadAllBytes(cropPath), expected,
                Path.GetFileName(cropPath));
        }

        // The in-memory form, called from the capture (public): same
        // single-pass rule, over crop bytes the caller chose not to
        // write to disk. context names the crop in warnings as its
        // would-be file name.
        public static bool CropConfirms(
            byte[] jpeg, int expected, string context)
        {
            int[] detected = Greyhound.Vision.Ghcv.Detect(jpeg, out string error);
            if (error != null)
            {
                Debug.LogWarning($"[ArUcoVerifier] {context}: {error}");
                return false;
            }
            return detected.Length == 1 && detected[0] == expected;
        }

        // One label file's confirmed lines, held until the refusal
        // decision - the rewrite happens only after the whole run is
        // accounted, because the never-wipe rule is global.
        private sealed class Rewrite
        {
            public string path;
            public List<string> kept;
        }
    }
}
