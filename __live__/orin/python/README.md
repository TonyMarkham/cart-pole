# The Orin's Python

The Orin has no monitor, so the only way to see what its camera sees is to send
a picture to a browser. `src/capture_push.py` takes one picture with the Global
Shutter camera and pushes it to every open viewer page. The same folder holds what
measures the calibration's stops and fits the camera's matrix, and the trained
policy (*The policy*).

It reads [settings.toml](settings.toml), and runs on the Orin only, because the
capture uses NVIDIA's camera software.

**There's nothing to build.** `uv` installs what the script needs into a `.venv`
folder beside it the first time it runs, leaving the Orin's own Python alone.

## The marker detector

The calibration has to find the markers in the real camera's frames, so this
folder has its own copy of the detector, made on 2026-11-01 from `__synthetic__/orin/python/src/`,
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
| `src/calibrate_service.py` | Runs one measurement at a time, and keeps each pose's result until the fit. Also runs the live view for the webserver's `/live` page, never at the same time as a measurement |
| `src/lens_calibration.py` | The lens measurement's arithmetic: the red box's size, the flat and turned positions, the board's shape at each, whether it is turned, and OpenCV's `calibrateCamera` on the kept stills |
| `src/charuco_board.py` | Makes the printable ChArUco board the lens is measured with (run on the PC, with `training/.venv`) |
| `src/measure.py` | Turns a pose's many detections of a marker into one point: drops the far-off ones, averages the rest |
| `src/lens.py` | Undoes the lens's bend, which the matrix alone can't follow, with the lens calibration in `lens.json`, halved for the detector's frames |
| `src/fit.py` | Works out where the seven poses put Pivot and End in metres, undoes their bend, fits the matrix, checks it and stores it |
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

**The seven poses** are the synthetic loop's (`synthetic-loop-dia.md`, *The seven
poses*): the pole hanging at each end and at the centre, and at each end also held
level, pointing away from the centre, and held straight up. The hanging poses alone
cover only the rail and the hanging pole, while a swinging End reaches the pole's
length, 0.450 m, past them on either side and above. Pivot is at its stop's `x` in
every pose; End is 0.450 m below it hanging, above it upright, and to the left of it
(idler) or the right (motor) held level.

**What survives a restart:** each pose's result and each end's gap are saved to
`kept_stops.json`, at `kept_path` in `settings.toml`. A file from before the seven poses
is not read, so they are all measured again. The settings are read afresh at each fit,
so a changed limit needs no restart either. The fit screen says how long ago each pose
was measured, because a pose kept from before the camera moved would spoil the fit:
measure all seven again after moving it.

**Where the stops come from:** real measurements win over the CAD. The gaps correct
where the stops sit against the CAD's walls, which gives the rail's centre, and the
board's measured span sets how far apart they are. The fit reports how far the CAD's
length is from the measured one, and that difference never affects the result. The
idler's end is on the left as the camera sees it, and the motor's on the right.

**The lens:** a wide-angle lens bows straight lines outward from the middle of the
picture, and the matrix can only map straight lines to straight lines. So each fit
first undoes every point's bend with the lens calibration (*Lens calibration*, below),
`lens.json`, halved for the detector's 728 x 544 frames (`src/lens.py`), and fits the
matrix to the straightened points. The fit refuses to run without `lens.json`. The 14
points then give the matrix's 8 numbers from 28 measurements, leaving 20 to check the
fit. Any later frame must have its bend undone with the same numbers before the matrix.
Until 2026-11-02 each fit instead searched for one number for the bend from these same
points, with the picture's centre standing in for the lens's; the seven-pose fit with it
failed, its worst point 3.59 px.

**What it writes:** `homography.json`, at `matrix_path` in `settings.toml`, with the
lens calibration used (its camera matrix and bend numbers, halved for the detector's
frames, its file and date), the matrix, each point and its residual, the span, and the
date. **Every fit is written, passed or not** (user, 2026-11-02): `"ok"` in the file says
whether every point is within `residual_px` of where the matrix puts it, so whatever
uses the matrix must check it. `homography.json` is the latest fit; a dated copy of each,
`saved/homography-<date>-<time>.json`, is never overwritten, so a passing fit is never
lost to a later failing one.

## Lens calibration

The camera's lens bends the picture, most at its edges: straight lines bow outward.
The matrix that turns pixels into metres can only map straight lines to straight
lines, so the bend has to be measured and undone first. Measuring it once, on its
own, is OpenCV's way: hold a printed board whose layout is known in many places and
at many angles, and OpenCV's `calibrateCamera` works out the lens from where the
board's corners land in each picture. The result holds until the lens or its focus
ring changes; moving the camera doesn't change it (OpenCV, "Calibration with ArUco
and ChArUco": the parameters "remain fixed unless the camera optic is modified").

**The board** is a ChArUco board: a chessboard with an ArUco marker in each white
square, so OpenCV knows which corner is which even when part of it is hidden. It has
6 x 4 squares, 15 inside corners, and its squares printed at 43 mm. `src/charuco_board.py`
makes it from `[lens_board]` in `settings.toml`; the printable files are in
[lens-board/](lens-board/).

**How it is done,** on a tablet at the rig, with the calibration service running:

1. Open <http://192.168.0.215:8080/lens/> and press **Start**.
2. Hold the board over the dashed white shape inside the red box, about 1 m from the
   camera. The box turns green when at least 11 of the 15 corners are found and all
   are inside it. Held green for 10 frames, the picture is kept as a *still* and the
   box moves on by itself.
3. The first 25 positions tile the whole picture with the board flat, facing the
   camera. The next 20 put the box at the centre and at each corner, and ask for the
   board turned so its left, right, top or bottom side, as the tablet shows it, is
   farther from the camera. The **Turn** bar shows the turn: 20 degrees counts, 30 is
   the aim. Turned stills are needed because flat ones, wherever they are held, all
   tell OpenCV the same thing about the lens (Zhang, "A Flexible New Technique for
   Camera Calibration", 2000, the method `calibrateCamera` is built on).
4. Press **Solve** once at least 10 stills are kept.

**Save picture** only saves what the camera sees, for looking at from elsewhere: it
keeps no still. **Skip this position** moves on without one. **Start over** forgets
every kept still.

**The files.** The service writes them on the Orin; copies of the 2026-11-02 run are
in [saved/](saved/) here, because nothing else on the PC holds them:

| File | On the Orin | What it is for |
|---|---|---|
| `lens.json` | `~/live-orin-python/lens.json` (`[lens] lens_path`) | **The result**, the file the rest of the calibration will read: the focal lengths and the lens's centre (`camera_matrix`, in full-sensor pixels, 1456 x 1088), the five bend numbers k1, k2, p1, p2, k3 (`dist`), the average error (`rms_px`), each still's error (`per_view_rms_px`), whether the bend grows steadily to the corners (`monotonic`), the board's printed square and the date |
| `lens-<date>-<time>.json` | `~/live-orin-python/saved/` | A dated copy of each Solve's result, never overwritten, so an earlier result can be gone back to |
| `lens_views.json` | `~/live-orin-python/lens_views.json` (`[lens] views_path`) | **The kept stills**: for each, the position it was kept at, how the board was turned, and every corner's pixel position and number. Saved as each still is kept, so a closed page or a restart loses none; Solve reads them, so it can be run again without the board. **Start over** empties it |

**The result on 2026-11-02**, from all 45 stills:

| | Value | What it means |
|---|---|---|
| Average error | 0.26 px; worst still 0.65 px | How far the board's corners are from where the result puts them. OpenCV gives no pass mark beyond closer to zero |
| Bend grows steadily | yes | OpenCV's calib3d reference: a bend that doesn't "should be considered a calibration failure" |
| Focal lengths fx, fy | 1846.9, 1837.6 px | Within 0.5% of each other, as square pixels should be |
| Lens centre cx, cy | 679.1, 468.4 px | 49 px left of and 76 px above the picture's centre, 728, 544 |
| Bend k1, k2, p1, p2, k3 | -0.519, +0.452, +0.014, +0.003, -0.417 | A strong barrel bend: lines bow outward, most at the edges |

**How the rig's calibration uses it:** each fit reads `lens.json` afresh, so a new
Solve is used at the next fit. The result is halved for the detector's 728 x 544
frames: fx, fy, cx and cy halved, while the bend numbers act on distances measured in
focal lengths, which halving leaves the same, so they stay. Pivot and End are
straightened with it before the matrix is fitted (*The calibration service*, *The
lens*), and the numbers used are stored in `homography.json` beside the matrix.

**`[lens] measured_marker_px` is wrong.** The 18.5 px it gives for the board's 32.25 mm
markers at 2 m means a focal length of 1147 px, and the solve found 1847, about 30 px
at 2 m. So the box, sized from it, fits the board held about 1.6 m away rather than
1 m, and the Turn bar's angles, worked out with 1147 px, were not exact. Neither goes
into the result: `calibrateCamera` uses only the corners.

**Moving this folder to the Orin** (*Move it to the Orin*) also carries `saved/` and
these copies into the Orin's `~/live-orin-python/saved/`. That changes nothing the
service reads, since it reads `lens.json` and `lens_views.json` from
`~/live-orin-python/` itself.

## The policy

**Why:** the policy is the small network that training learned in the synthetic
loop: it reads what the camera saw and picks the cart's next command. The Orin runs
it to balance the real pole, but training saved it in Stable-Baselines3's format,
which needs PyTorch, and nothing is installed on the Orin without asking. So the
trainer writes the part the rig needs into one numpy file, `saved/policy.npz`, and
`src/policy.py` works out the command from it with numpy alone, which the Orin
already has.

**What the rig needs of it.** Training used SAC, which trains two kinds of network:
the **actor**, which picks the command, and the **critics**, which only teach the
actor. The rig keeps the actor, and only its best guess, with no randomness:

```text
h1   = relu(W1 x + b1)      x: the 36 inputs; W1 is 256 x 36
h2   = relu(W2 h1 + b2)     W2 is 256 x 256
mean = W3 h2 + b3           W3 is 1 x 256
a    = tanh(mean)           a is in [-1, 1]
command = round(a x 44,600) pulses/s
```

`relu(v)` is max(v, 0), for each number on its own; `tanh` squeezes any number
into [-1, 1]. The 36 inputs are the pool's 6 entries, newest first, each its age
in ms ÷ 1,000 and Pivot's and End's (x, z) in metres, (-1, -1) for a marker not
found, then the last 6 values of a, newest first ([real-loop-dia.md](../../../real-loop-dia.md),
*The pool on the rig*).

**What is in the file:**

| Name | What it is |
|---|---|
| `hidden_1_weight`, `hidden_1_bias`, `hidden_2_weight`, `hidden_2_bias`, `output_weight`, `output_bias` | W1, b1, W2, b2, W3, b3 above |
| `top_pulse_rate` | 44,600 pulses/s: a = 1 asks for it |
| `age_unit_ms` | 1,000: each age is divided by it |
| `past_commands` | 6: how many past values of a the inputs end with |
| `command_rate_min_hz`, `command_rate_max_hz` | 20 and 30: the band of command rates it trained with |
| `check_inputs`, `check_actions` | 1,000 inputs from the run, with Stable-Baselines3's own a for each |
| `source` | the run, the save and the commit it came from |

The values come from the trained run's manifest, so the brain cannot build the
inputs or the command differently from training.

**Today's file** is run `20261101_163458`'s last save, 4,000,008 steps, commit
`9087b8fd`: the run that swings the pole up and balances it in the synthetic loop
([trainer-observations.md](../../../trainer-observations.md)). Before writing it,
the export worked out a for all 1,999,992 inputs in the run's replay buffer both
with Stable-Baselines3 and with the formula above; the worst difference was
0.0000043 (2026-11-02).

**To write it again,** from the repo root on the PC, then copy it here:

```bash
uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/export_policy.py __synthetic__/trainer/python/runs/20261101_163458
```

```bash
cp __synthetic__/trainer/python/runs/20261101_163458/policy.npz __live__/orin/python/saved/policy.npz
```

**To check it,** here or on the Orin. This works out the 1,000 stored inputs with
numpy and compares them with Stable-Baselines3's answers; the PC gave a worst
difference of 0.000001:

```bash
uv run python src/test_policy.py
```

Nothing runs the policy yet: the brain, which times the commands, builds the
inputs and sends the commands, is not built.

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
