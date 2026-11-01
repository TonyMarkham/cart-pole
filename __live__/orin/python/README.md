# The Orin's Python

The Orin has no monitor, so the only way to see what its camera sees is to send
a picture to a browser. `src/capture_push.py` takes one picture with the Global
Shutter camera and pushes it to every open viewer page. The same folder holds what
measures the calibration's stops and fits the camera's matrix.

It reads [settings.toml](settings.toml), and runs on the Orin only, because the
capture uses NVIDIA's camera software.

**There's nothing to build.** `uv` installs what the script needs into a `.venv`
folder beside it the first time it runs, leaving the Orin's own Python alone.

## The marker detector

The calibration has to find the markers in the real camera's frames, so this
folder has its own copy of the detector, made on 2026-11-01 from `orin/src/`,
because `__live__` can use nothing outside itself.

| File | What it does |
|---|---|
| `src/detector.py` | Takes one JPEG and returns where each of the four markers is, or that it wasn't found. It doesn't care where the JPEG came from |
| `src/detector_config.py` | Reads `[detector]` from `settings.toml` |
| `src/build_engine.py` | Builds the TensorRT engine, the model compiled for this GPU, from the model's ONNX file |
| `src/reference_check.py` | Checks that the detector gets the same answers as Ultralytics' own predictor did on the same frames |
| `src/camera.py` | Gives the real camera's frames, halved to 728 x 544 greyscale, the format of the real images the model has already been run on |
| `src/live_detect.py` | Prints what the detector sees through the real camera, for checking by hand |

The two files that get frames from the Unity player, `synthetic_orin.py` and
`frame_socket.py`, stayed behind, because the real rig has no player.

**The detector needs the `.venv` to see the system's packages,** because TensorRT
is installed only for the system's Python. `capture_push.py` doesn't mind either
way. On the Orin, replace the `.venv` once with one that can:

```bash
cd ~/live-orin-python && rm -rf .venv && uv venv --system-site-packages --python /usr/bin/python3.10
```

The model and its engine aren't in git or in the package. `settings.toml` names the
copies already on the Orin.

**To see it working,** with nothing else using the camera. Nothing moves. The file
named at the end gets the first frame, to look at:

```bash
cd ~/live-orin-python && .venv/bin/python src/live_detect.py 120 /tmp/live.jpg
```

On 2026-11-01 this found all four markers in 120 of 120 frames, with Pivot steady to
about 0.03 px.

## The calibration service

The camera's pixels only become metres through a matrix, and fitting it takes the
camera and the GPU, which a browser can't reach. So `src/calibrate_service.py` is a
small service that owns them. The calibration page asks the webserver, and the webserver
asks this. It only listens to the Orin itself, and it can't move the cart.

| File | What it does |
|---|---|
| `src/calibrate_service.py` | Runs one measurement at a time, and keeps each stop's result until the fit |
| `src/measure.py` | Turns a stop's many detections of a marker into one point: drops the far-off ones, averages the rest |
| `src/lens.py` | Undoes the lens's bend, which the matrix alone can't follow |
| `src/fit.py` | Works out where the stops are in metres, finds the lens number, fits the matrix, checks it and stores it |
| `src/test_calibration.py` | Checks that arithmetic with known answers, with no camera or GPU |

**Start it,** after replacing the `.venv` as above. It carries on after the SSH session
ends, doesn't start by itself after a reboot, and its messages go to `service.out`:

```bash
cd ~/live-orin-python && (nohup setsid .venv/bin/python src/calibrate_service.py > service.out 2>&1 < /dev/null &)
```

`service.out` should end with `detector loaded`. The camera opens only during a
measurement, so `capture_push.py` and `live_detect.py` can run between measurements.

**To stop it,** for example before restarting it after a change:

```bash
pkill -f calibrate_service.py
```

**To check the arithmetic:**

```bash
cd ~/live-orin-python && .venv/bin/python src/test_calibration.py
```

**The two hand measurements are required.** With the cart stopped on each switch,
you measure the gap from the cart's face to the rail support it faces, and type it into
the calibration page. A measurement at an end won't start without one.

**What survives a restart:** each stop's result and gap are saved to `kept_stops.json`,
at `kept_path` in `settings.toml`. The settings are read afresh at each fit, so a changed
limit needs no restart either. The fit screen says how long ago each stop was measured,
because a stop kept from before the camera moved would spoil the fit: measure all three
again after moving it.

**Where the stops come from:** real measurements win over the CAD. The gaps correct
where the stops sit against the CAD's walls, which gives the rail's centre, and the
board's measured span sets how far apart they are. The fit reports how far the CAD's
length is from the measured one, and that difference never affects the result. The
idler's end is on the left as the camera sees it, and the motor's on the right.

**The lens:** a wide-angle lens bows straight lines outward from the middle of the
picture, and the matrix can only map straight lines to straight lines. The six points'
real positions are known, three columns the span apart and two rows the pole's length
apart, so each fit also finds the one number for that bend that leaves the matrix
closest to them. On 2026-11-01 the worst point went from 1.600 px to 0.166 px, about
0.4 mm, with the number at +0.44. Those 9 fitted numbers come from 12 measurements, so
the six points look good partly because they were fitted. Any later frame must have
the same correction applied before the matrix.

**What it writes:** `homography.json`, at `matrix_path` in `settings.toml`, with the
lens number, the matrix, each point and its residual, the span, and the date. It is written only when
every point is within `residual_px` of where the matrix puts it.

## What it needs on the Orin

- `uv`, which is at `~/.local/bin/uv` on `robo-orin`.
- `gst-launch-1.0` and NVIDIA's camera software, which come with JetPack, the
  Orin's operating system.
- **The webserver running, and `image-push` built,** both in `~/orin-cam`
  ([their README](../rust/README.md)). `settings.toml`'s `[push]`
  table names them.

## Move it to the Orin

On the PC, in Git Bash, from this folder, pack it. The PC's `.venv` stays behind,
because it wouldn't work on the Orin:

```bash
tar --exclude=./.venv -czf ../../../target/live-orin-python.tgz .
```

Move `target/live-orin-python.tgz` from the repo's root to the Orin's home folder
([how](../../README.md)). On the Orin, unpack it. Files of the same name are
replaced and nothing is deleted:

```bash
mkdir -p ~/live-orin-python && tar -xzf ~/live-orin-python.tgz -C ~/live-orin-python
```

This is where it is on `robo-orin`, since 2026-10-31. An older temporary copy in
`/tmp/live-orin-python` is no longer needed.

## Run it

On the Orin, with a viewer open at <http://192.168.0.215:8080/viewer/>:

```bash
cd ~/live-orin-python && uv run python src/capture_push.py
```

It takes about 8 s, nearly all of it writing the frames. The last lines it prints
are these, and the picture appears in the viewer:

```text
Pushed /tmp/orin-capture.png
pushed to http://localhost:8080
```
