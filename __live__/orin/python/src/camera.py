"""The real camera as a stream of frames, for the detector: the sensor's full frame
halved to 728 x 544 greyscale.

Runs on the Orin only: NVIDIA's camera software through GStreamer
(gst-launch-1.0, a program that chains video steps together). Each frame arrives
as raw bytes, one byte per pixel, so every frame is exactly width x height bytes
and needs no searching for where it ends.

The frames come down a pipe of their own, not the program's standard output,
because the camera software prints its messages there, and one stray line would
shift every frame after it.
"""

import os
import signal
import subprocess
from typing import BinaryIO, Iterator

# How long the camera software gets to shut down cleanly when asked, s. Killed
# outright, it can leave NVIDIA's camera service unable to open the camera again.
STOP_TIMEOUT_S = 5


class CameraError(RuntimeError):
    """The camera stopped sending frames."""


class Camera:
    """Use with `with`, so the camera is always let go of."""

    def __init__(self, camera: dict, stream: dict):
        self.width = stream["width"]
        self.height = stream["height"]
        self._camera = camera
        self._stream = stream
        self._process: subprocess.Popen | None = None
        self._frames: BinaryIO | None = None

    def __enter__(self) -> "Camera":
        read_end, write_end = os.pipe()
        camera = self._camera
        command = [
            "gst-launch-1.0", "-q",
            "nvarguscamerasrc",
            f"sensor-id={camera['sensor_id']}",
            f"sensor-mode={camera['sensor_mode']}",
            f"tnr-mode={camera['tnr_mode']}",
            f"ee-mode={camera['ee_mode']}",
            # Locked, because automatic exposure would smear the swinging pole's end
            # and change the picture from frame to frame.
            f"exposuretimerange={self._stream['exposure_ns']} {self._stream['exposure_ns']}",
            f"gainrange={self._stream['gain']} {self._stream['gain']}",
            "ispdigitalgainrange=1 1",
            "!", f"video/x-raw(memory:NVMM),width={camera['width']},height={camera['height']},framerate={camera['framerate']}/1",
            # Out of the camera's GPU memory, halved, one byte per pixel.
            "!", "nvvidconv",
            "!", f"video/x-raw,format=GRAY8,width={self.width},height={self.height}",
            "!", "fdsink", f"fd={write_end}",
        ]
        self._process = subprocess.Popen(command, pass_fds=(write_end,), stdout=subprocess.DEVNULL)
        # Only the camera software holds the write end now, so the pipe ends when it does.
        os.close(write_end)
        self._frames = os.fdopen(read_end, "rb")
        return self

    def __exit__(self, *_) -> None:
        if self._process is not None and self._process.poll() is None:
            self._process.send_signal(signal.SIGINT)
            try:
                self._process.wait(STOP_TIMEOUT_S)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait()
        if self._frames is not None:
            self._frames.close()

    def frames(self) -> Iterator[bytes]:
        """Each frame's pixels, top row first, until the camera stops."""
        size = self.width * self.height
        while True:
            frame = self._frames.read(size)
            if len(frame) != size:
                raise CameraError(f"the camera stopped: got {len(frame)} of {size} bytes")
            yield frame
