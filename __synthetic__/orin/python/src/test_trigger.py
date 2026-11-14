"""The trigger forwards the camera's pose only when the orchestrator sent one
(__documentation__/synthetic-loop-dia.md, Contracts, the camera pose)."""

import json
import unittest

from trigger import trigger

JOINTS = [{"name": "cart", "qpos": 0.05}]
CAMERA = {"position": {"x": 0.0, "y": 1.1, "z": -2.5}, "eulerRotation": {"x": 10.0, "y": 0.0, "z": 0.0}}


class Trigger(unittest.TestCase):
    def test_without_a_camera_pose_only_the_id_and_joints_go(self):
        pose = {"id": 7, "time": 0.25, "joints": JOINTS}
        self.assertEqual(json.loads(trigger(7, pose)), {"id": 7, "joints": JOINTS})

    def test_a_camera_pose_is_forwarded_unchanged(self):
        pose = {"id": 7, "time": 0.25, "joints": JOINTS, "cameraTransform": CAMERA}
        self.assertEqual(json.loads(trigger(7, pose)), {"id": 7, "joints": JOINTS, "cameraTransform": CAMERA})

    def test_a_measure_message_triggers_with_its_own_frame_id(self):
        measure = {"type": "measure", "pose": "left up", "joints": JOINTS, "pivot": [0, 0], "end": [0, 0],
                   "cameraTransform": CAMERA}
        self.assertEqual(json.loads(trigger(12, measure)), {"id": 12, "joints": JOINTS, "cameraTransform": CAMERA})


if __name__ == "__main__":
    unittest.main()
