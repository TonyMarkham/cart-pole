using System;

namespace CartPole.Vision
{
    // One class's verification identity: the class id (rig slot
    // order, the YOLO label's first field), the ArUco tag id its
    // labels claim (DICT_4X4_1000), and the label used in the
    // dropped-markers report. Built by the caller from the rig's
    // authored Marker SOs.
    public sealed class ArUcoVerifierEntry
    {
        public int classId;
        public int tagId;
        public string label;
    }
}
