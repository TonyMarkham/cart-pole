"""Shows what the real camera sees, on a browser, because the Orin has no
monitor: takes one picture with the Global Shutter camera and pushes it to
every open viewer page with orin-cam's image-push tool.

Runs on the Orin only: the capture uses NVIDIA's camera software through
GStreamer (gst-launch-1.0, a program that chains video steps together). The
picture is full colour, not the detector's 728 x 544 greyscale.

The settings are settings.toml, beside this folder.

usage (from __live__/orin/python): uv run python src/capture_push.py
"""

import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

SETTINGS = Path(__file__).resolve().parent.parent / "settings.toml"


def capture(camera: dict, png: dict) -> None:
    """Writes the frames to one file name, each replacing the one before, so
    the file left at the end is the last frame."""
    command = [
        "gst-launch-1.0", "-q",
        "nvarguscamerasrc",
        f"sensor-id={camera['sensor_id']}",
        f"sensor-mode={camera['sensor_mode']}",
        f"num-buffers={camera['frames']}",
        f"tnr-mode={camera['tnr_mode']}",
        f"ee-mode={camera['ee_mode']}",
        "!", f"video/x-raw(memory:NVMM),width={camera['width']},height={camera['height']},framerate={camera['framerate']}/1",
        # Out of the camera's GPU memory into plain memory the PNG step reads.
        "!", "nvvidconv",
        "!", "video/x-raw,format=RGBA",
        "!", "pngenc", f"compression-level={png['compression_level']}",
        "!", "multifilesink", f"location={png['path']}",
    ]
    subprocess.run(command, check=True, timeout=camera["timeout_s"])


def push(settings: dict, png_path: str) -> None:
    tool = str(Path(settings["image_push"]).expanduser())
    subprocess.run([tool, "--server", settings["server"], png_path], check=True)


def main() -> int:
    settings = tomllib.loads(SETTINGS.read_text())
    png_path = settings["png"]["path"]
    try:
        capture(settings["camera"], settings["png"])
        print(f"captured {png_path} ({Path(png_path).stat().st_size} bytes)", flush=True)
        push(settings["push"], png_path)
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired, OSError) as error:
        print(f"failed: {error}", file=sys.stderr)
        return 1
    print(f"pushed to {settings['push']['server']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
