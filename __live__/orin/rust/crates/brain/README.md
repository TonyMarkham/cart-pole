# brain — the balance loop on the rig

The trained policy balances nothing until a program on the robot's own
computer does what the synthetic loop did for it: take a camera frame, find
the markers, keep the pool, ask the policy for a command and send it to the
Arduino, over and over until the person says stop. `brain` is that program,
on the Jetson Orin Nano Super. It is its own program, a Rust orchestrator
(user, 2026-11-04), and it calls Python only for the two things that have to
be Python on the robot: finding the markers, with TensorRT, and the policy's
inference, with `policy.py`. Its design is `__documentation__/real-loop-dia.md`, the single
source of truth for the real loop; this page says what the program does and
how it is run. **Built on 2026-11-07; first run on the rig on 2026-11-09**,
where the 6M-step policy swung the pole up in 3.4 s and held it within 10° of
upright for the rest of a 30 s run.

```text
/run page  --POST /run/start-->  webserver  --POST /run-->  brain  --F frame-->   brain_worker.py (TensorRT, policy.npz)
                                                              |    <--boxes--
                                                              |    --P inputs-->
                                                              |    <--a--
                                                              +--UDP 3 bytes-->  Arduino
```

## What one run is

1. **Run.** The webserver forwards the page's Run to the brain. The brain
   refuses it, saying why, while a run is going, while `calibrate_service.py`
   has the camera (`GET /busy`), or while the matrix in `homography.json` did
   not pass and was not accepted. Otherwise it makes the run's folder, reads
   the matrix and the lens stored with it, pre-warms the pool with six empty
   entries 32 ms apart, opens the camera and drops its settling frames.
2. **Every command, until STOP or a FAULT:**
   - the camera's next frame after the wait, 728 × 544 greyscale, with
     GStreamer's timestamp; frames that arrived during the wait are dropped
     unread;
   - the frame to the worker, which encodes it as JPEG quality 90, decodes it
     again (as the training frames were made) and runs the detector: per class
     the box's centre, size and confidence, or (−1, −1) for a marker not found;
   - Pivot's and End's centres to pixels, the lens's bend undone, then through
     the matrix: (x, z) in metres in the pole's plane;
   - the pool: the new entry at age 0, every older entry aged by the time
     between the two frames' stamps, the oldest dropped;
   - the 36 inputs to the worker: the pool's 30 numbers, ages in seconds, then
     the last 6 actions, newest first; a = tanh(mean) back;
   - c = round(a × 44,600), sent to the board as 3 bytes: the sign (0
     negative, 1 positive), then the size as a `u16`, little-endian;
   - one line in `frames.jsonl`;
   - any reply from the board: the FAULT STATE or UNCALIBRATED reply ends the
     run;
   - the wait, `delay_ms`.
3. **STOP.** The webserver asks the brain for idle and waits until the loop
   has stopped, so no command follows; only then does it send the board the
   command 0 several times. The brain closes the camera and the log.

A failed frame, one the detector refuses, is an entry of (−1, −1) and ends
nothing. There is no time limit and no gap check: the board's soft limit keeps
the cart from the switches, and its FAULT is what ends a run early.

## The worker

`__live__/orin/python/src/brain_worker.py`, started by the brain as a child
process when the brain starts, in the Python project's folder, with the command
in `[brain] worker`. It loads the TensorRT engine and `saved/policy.npz` once,
writes one JSON line with the policy's values (the top pulse rate, the age unit,
the past commands, the run it came from), and then answers
requests on its pipes until the brain closes them:

| In | Out |
|---|---|
| `F` then the frame's 396,032 bytes (728 × 544) | 80 bytes: per class in class order, centre x, centre y, width, height, confidence, each a little-endian `f32`; (−1, −1, 0, 0) for a marker not found |
| `P` then 36 little-endian `f64` | a, one little-endian `f64` |

The brain does not start until the ready line arrives, so a Run press never
waits for the engine to load and never fails on it. Everything the worker
prints goes to standard error; standard output is the brain's.

## Settings

`settings.toml`, in `__live__/orin/rust`, the file the webserver reads too.
Each value is explained where it is set:

- **`[brain]`:** `address`, where it answers the webserver; `worker_dir` and
  `worker`, the Python folder and the command; `delay_ms`, the wait after each
  command; `pre_warm_gap_ms`, the empty pool's gaps; `runs_dir`, where the
  logs go; `homography`, the matrix's file; `service`, where
  `calibrate_service.py` answers `/busy`.
- **`[camera]`:** the sensor's mode, shutter and gain, the same values as the
  Python's `[camera]` and `[stream]`, because the matrix was fitted on the
  frames those made.
- **`[arduino]`:** `address` and `port`, where the board listens.

## Requests

On `[brain] address`, the Orin's own address (`127.0.0.1:8091`); the webserver's
`/run/start`, `/run/stop` and `/run/status` are the way to them from a page.

| Route | Method | Answer |
|---|---|---|
| `/run` | POST | 200 and the status once the run has started, or 409 with `message` saying why not |
| `/idle` | POST | 200 and the status once the loop has stopped, so no command follows |
| `/status` | GET | `state` (`"run"` or `"idle"`), `message` (why the last run ended, or why Run was refused), `run_dir`, `frames` |

## The log

`runs_dir/YYYYMMDD_HHMMSS/frames.jsonl` (UTC, the Run press), one JSON object
per frame, flushed as each is written: `boxes`, `points`, `inputs`, `stamp`
(GStreamer's, ns), `a`, `c`, and `arrived` and `sent`, milliseconds since the
Run press on the Orin's monotonic clock. Their difference is the brain's own
share of the delay D (`__documentation__/real-loop-dia.md`, *Measuring D*).

## The camera

The brain runs `gst-launch-1.0` with the same pipeline as the Python's
`camera.py`, ending in `gdppay ! fdsink fd=1`: the GStreamer Data Protocol
writes each frame down the pipe behind a 62-byte header that carries the
buffer's timestamp, so the stamp and the pixels arrive together. It needs the
`gdp` plugin, from `gstreamer1.0-plugins-bad`; on the Orin:

```bash
gst-inspect-1.0 gdppay
```

A reader thread keeps only the newest frame, so the pipe never fills and the
camera software never waits on the brain. When the run ends the camera software
is asked to stop with an interrupt, as the Python does, and killed only if it
has not stopped in 5 s.

## Running it

Built and started on the Orin beside the webserver (`../../DEPLOY.md`), from
`~/orin-cam`, so it finds `settings.toml`. It needs the Python project at
`[brain] worker_dir` with its environment made, the TensorRT engine at the
Python's `[detector] engine`, `saved/policy.npz`, and `homography.json` from a
calibration. Its first lines say what it loaded:

```text
worker ready: policy 20261108_115227/latest.zip, 6,000,000 steps, ..., top rate 44600 pulses/s, 6 past commands, ages in units of 1000 ms
brain idle at http://127.0.0.1:8091
```

**Starting and stopping a run.** From the `/run` page,
<http://192.168.0.215:8080/run/>: its STOP is always pressable, and its Run
only while the brain is idle and the board answers `state` with
`"calibrated"`, with the reason beside a greyed-out Run otherwise. Before
Run, the board must be calibrated (the seeks and the move to the centre on
the `/calibrate` page), `homography.json` must pass or be accepted, and the
pole should hang still. The page reads the state when it opens and after each
press. Its Re-centre moves the cart back to the rail's centre after a run, with
the board's `MoveToCentre`; its STOP also sends `Stop`, which ends that move.

The same requests from the PC, without the page, in PowerShell (`curl.exe`,
since PowerShell's own `curl` is another command):

```powershell
curl.exe -s -X POST http://192.168.0.215:8080/run/start
```

```powershell
curl.exe -s -X POST http://192.168.0.215:8080/run/stop
```

```powershell
curl.exe -s http://192.168.0.215:8080/run/status
```

The status's `message` says why the last run ended: `stopped`, or the board's
FAULT, such as `the board is in FAULT, at 58839 pulses: reset it, then
calibrate` when the cart reached a limit switch. While the brain is idle the
webserver adds the board's own answer to `state` as `board`, or `board_error`
saying why there is none; during a run the brain alone talks to the board.

Checks that need no rig, from `__live__/orin/rust` on the PC:

```powershell
cargo clippy --workspace --all-targets --no-deps
```

```powershell
cargo test -p brain
```

And the worker's framing, from `__live__/orin/python`:

```bash
uv run python src/test_brain_worker.py
```

## What is not yet done

- D is not measured (`__documentation__/real-loop-dia.md`, *Measuring D*).
