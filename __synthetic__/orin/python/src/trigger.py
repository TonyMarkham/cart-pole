"""The trigger (__documentation__/synthetic-loop-dia.md, 12): the text message that asks
the player for one frame.

It is the orchestrator's pose forwarded as is, less what the player does not use: the
id, the joints, and the camera's pose when the orchestrator sent one (Contracts, the
camera pose). Without one the key is left out, and the player's camera stays where it
is. Example:

    {"id": 7, "joints": [{"name": "cart", "qpos": 0.05}],
     "cameraTransform": {"position": {"x": 0, "y": 1.1, "z": -2.5},
                         "eulerRotation": {"x": 0, "y": 0, "z": 0}}}
"""

import json

CAMERA_TRANSFORM = "cameraTransform"


def trigger(frame_id: int, pose: dict) -> str:
    """The trigger for `pose` (a pose or measure message), numbered `frame_id`."""
    message = {"id": frame_id, "joints": pose["joints"]}
    if CAMERA_TRANSFORM in pose:
        message[CAMERA_TRANSFORM] = pose[CAMERA_TRANSFORM]
    return json.dumps(message)
