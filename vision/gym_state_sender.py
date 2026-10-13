"""Bridge client: the gym (Python) side of the Unity vision system.

The training loop steps MuJoCo, reads the true joint state from
``data.qpos`` (fixed sim clock: step N is at exactly N * model.opt.timestep),
and pushes it to the Unity StateServer as JSON. Unity applies the state to
the imported assembly, renders, and returns a JPEG frame.

Protocol: one JSON object per line in; a 4-byte little-endian length header
plus JPEG bytes out.

    {"id": 7, "joints": [{"name": "02-Cart.Slider.joint", "qpos": 0.05},
                          {"name": "04-Pole.Hinge.joint", "qpos": 0.3}]}

Usage inside a gymnasium/MuJoCo training loop:

    from gym_state_sender import UnityVisionClient

    vision = UnityVisionClient()
    joint_names = ("02-Cart.Slider.joint", "04-Pole.Hinge.joint")

    obs, info = env.reset()
    for step in range(N):
        action = policy(obs)
        obs, reward, terminated, truncated, info = env.step(action)

        if step % RENDER_EVERY == 0:
            qpos = env.unwrapped.data.qpos
            frame = vision.send_state(
                {name: float(qpos[i]) for i, name in enumerate(joint_names)},
                frame_id=step,
            )
            # frame is JPEG bytes; decode with cv2.imdecode or PIL
"""

import json
import socket
import struct


class UnityVisionClient:
    def __init__(self, host="127.0.0.1", port=5555, timeout=10.0):
        self.address = (host, port)
        self.timeout = timeout
        self._socket = None

    def __enter__(self):
        self.connect()
        return self

    def __exit__(self, *exc):
        self.close()

    def connect(self):
        self._socket = socket.create_connection(self.address, timeout=self.timeout)
        self._socket.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)

    def close(self):
        if self._socket is not None:
            try:
                self._socket.close()
            finally:
                self._socket = None

    def send_state(self, joints, frame_id=0):
        """joints: {name: qpos}. Returns the rendered JPEG bytes."""
        payload = json.dumps({
            "id": frame_id,
            "joints": [{"name": name, "qpos": value} for name, value in joints.items()],
        }).encode("utf-8") + b"\n"
        self._socket.sendall(payload)
        length = self._recv_exact(4)
        (size,) = struct.unpack("<I", length)
        return self._recv_exact(size)

    def _recv_exact(self, count):
        chunks = []
        received = 0
        while received < count:
            chunk = self._socket.recv(min(65536, count - received))
            if not chunk:
                raise ConnectionError("Unity closed the connection")
            chunks.append(chunk)
            received += len(chunk)
        return b"".join(chunks)


if __name__ == "__main__":
    # smoke test against a running Unity StateServer
    import sys
    import time

    with UnityVisionClient() as client:
        joints = {"02-Cart.Slider.joint": 0.0, "04-Pole.Hinge.joint": 0.5}
        for i in range(10):
            joints["02-Cart.Slider.joint"] = 0.4 * (i / 9.0) - 0.2
            joints["04-Pole.Hinge.joint"] = 0.3 * (i / 9.0)
            frame = client.send_state(joints, frame_id=i)
            print(f"frame {i}: {len(frame)} jpeg bytes")
            time.sleep(0.05)
