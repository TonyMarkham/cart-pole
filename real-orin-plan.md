# Real Orin Plan: the brain, the webserver and its pages

The trained policy balances nothing until a program on the robot's own computer
does what the synthetic loop did for it: take a camera frame, find the markers,
keep the pool, ask the policy for a command and send it to the Arduino, over and
over until the person says stop. That program is the **brain**, a Rust
orchestrator on the Jetson Orin Nano Super (user, 2026-11-04). It calls Python
only for what has to be Python or is already built there: finding the markers,
and the policy's inference. The person drives it from pages on the Orin's Rust
webserver, which also gets an Arduino Settings page for the board's one saved
setting.

Status: **plan** (2026-11-03; rewritten for the Rust brain 2026-11-07), written
from `real-loop-dia.md` alone, the source of truth. Where this plan and the
diagram disagree, the diagram is right. Section names in *italics* are the
diagram's.

## Done when

1. **Run starts a run and only STOP or a FAULT ends it.** From idle, Run opens
   the camera and starts sending commands; the run goes on through failed
   frames and through the soft limit until STOP, or until the board's FAULT
   STATE reply; after either the brain is idle and sends nothing.
2. **The inputs match training's.** For a logged frame, the 36 inputs can be
   rebuilt by hand from the log's coordinates, its capture times and the past
   actions: the pool newest first, ages ÷ 1,000, then the last 6 actions,
   newest first.
3. **The command is the policy's.** For logged inputs, `policy.npz` through
   numpy gives the same a, and c = round(a × 44,600).
4. **The ages are GStreamer's.** Each frame's age comes from its
   `do-timestamp` stamp, not from the time the brain read it.
5. **Each run has its log.** `runs/<the Run press's timestamp>/frames.jsonl`
   holds, per frame, the model's boxes, the coordinates after the homography,
   the 36 inputs, the frame's stamp, a and c, and `arrived` and `sent`.
6. **`runtime_end_margin` is set and read from the Arduino Settings page.**

## Requirements (from the diagram)

The brain (Rust, `__live__/orin/rust/crates/brain`, the Orin's own workspace,
built on the Orin) and its worker (Python,
`__live__/orin/python/src/brain_worker.py`):

- **O1: run and idle** (decided 2026-11-03). The brain holds a run/idle state;
  the webserver sends it requests to change state, as it does for
  `calibrate_service.py`, on the Orin's own address. Run starts one run; STOP
  or a FAULT ends it. No time limit, no episodes, no gap check (*The run and
  idle states*).
- **O2: at Run** (*The sequence*): refuse while `calibrate_service.py` is
  busy or the matrix is not ok, saying why; open the run's log folder; read
  the matrix (`homography.json`) and the lens stored with it; pre-warm the
  pool with six entries of (−1, −1) at `pre_warm_gap_ms` gaps and the last 6
  actions as 0; open the camera and drop the settling frames.
- **O3: the trained values from `policy.npz` alone** (*The policy on the
  Orin*): the top pulse rate, the age unit and the past commands, as
  `policy.py` loads them. They reach the brain in the worker's ready line. The
  command band left the file on 2026-11-09.
- **O4: each command** (*The sequence*, the loop):
  1. the camera's next frame, 728×544 greyscale, with GStreamer's stamp, read
     by the brain through `gdppay`;
  2. the frame to the worker, which does JPEG q90 then decode, as the training
     frames were made, and detects: per class the model's box, or (−1, −1);
  3. in the brain, Pivot and End: scaled to pixels, the lens's bend undone,
     then through the matrix, (x, z) in metres;
  4. push onto the pool at age 0; every older entry ages by the time since the
     last capture;
  5. the 36 inputs to the worker; a = tanh(mean), no randomness, back; c =
     round(a × the top pulse rate);
  6. send c as one 3-byte datagram to `192.168.0.220:4210`: byte 0 the sign
     (0 negative, 1 positive), bytes 1–2 the size of c as a `u16`,
     little-endian (decided 2026-11-03);
  7. one log line;
  8. any reply from the board that is the FAULT STATE or UNCALIBRATED reply
     ends the run, idle, with its reason kept for `/run`;
  9. wait `[brain] delay_ms`, then take the camera's next frame; frames before
     it are dropped unread (*Timing*).
- **O5: failed frames end nothing.** A frame the worker cannot decode is an
  entry of (−1, −1) and the run goes on (*The run and idle states*).
- **O6: GStreamer's timestamp** (decided 2026-11-03). `nvarguscamerasrc` with
  `do-timestamp`, not Argus's sensor timestamp. The brain reads each frame
  behind `gdppay`'s header, which carries the stamp (*The pool on the rig*).
- **O7: the log** (decided 2026-11-03; the format, *The log*). One folder per
  run under the runs directory on the Orin, named by the timestamp of the Run
  press; `frames.jsonl`, one line per frame, flushed as it happens.
- **O8: the worker** (proposed, *The run and idle states*). A child of the
  brain, started with it, holding the TensorRT engine and `policy.npz` for the
  brain's whole life; the brain does not start without its ready line. It
  speaks the fixed-size pipe protocol of *Contracts*, the brain's worker.

The webserver (Rust, `__live__/orin/rust/crates/webserver`) and its pages
(Rust compiled to WebAssembly, `__live__/orin/wasm`; no hand-written
JavaScript, `AGENTS.md`):

- **O9: Run and STOP.** `/run/start` forwards a run request to the brain.
  `/run/stop` asks the brain for idle first, so no command follows, then sends
  the board the command 0 `stop_sends` times, whether or not the brain
  answered. `/run/status` forwards the brain's status (*The run and idle
  states*, two ways out of run; *Contracts*, the brain's requests).
- **O10: the camera's one rule, the webserver's end.** `/live/start`,
  `/lens/start` and `/calibrate/measure` are refused while the brain reports
  a run (*The run and idle states*).
- **O11: the Arduino Settings page** (decided 2026-11-03). It sets
  `runtime_end_margin` on the board and shows the value the board holds
  (*The Uno's settings*). A setting is changed only when no run is going
  (follows).
- **O12: the viewer's Refresh button** (the user, 2026-11-08). The viewer
  page gets a button that puts one fresh camera frame on it.
  `/image/refresh` starts the live view through `calibrate_service.py`,
  asks its status until it carries a frame taken since the start, stops
  the live view if it was off before, and pushes that frame to every open
  viewer, as `/image` does a PNG. Refused while the brain reports a run,
  as `/live/start` is (O10).

Rules for both:

- **Tunable values in a toml** (`AGENTS.md`): the brain's and the webserver's
  in `__live__/orin/rust/settings.toml`, the one file both read from the
  folder they run in: `[brain]` (its address, the worker's command and folder,
  `delay_ms`, `pre_warm_gap_ms`, the runs directory, the matrix's path, the
  service's address) and `[camera]` (the sensor's mode, shutter and gain, the
  same values as the Python's `[camera]` and `[stream]`, since the matrix was
  fitted on those frames). The worker reads the Python `settings.toml` for the
  detector, as the service does.
- **Live and synthetic share no code** (`AGENTS.md`): the pool's rules are the
  synthetic Orin's (`__synthetic__/orin/python/src/synthetic_orin.py`, the
  rules), copied into Rust.
- **Clippy clean, no `unwrap()` outside tests** (`AGENTS.md`), checked from
  `__live__/orin/rust`.

## Not decided by the diagram

Each lands in the diagram first (`real-loop-dia.md`, the *Open* lists):

- **The five points of issue 3's plan** are proposed in the diagram and built
  as proposed (*The run and idle states*); the user has not decided them.
- **Which page carries Run and STOP** is decided, `/run` (issue 2); the page
  is built (step 9).
- **The settings messages** to the board and their answers. This blocks step
  10.
- **D**, the delay, not measured as a whole (*Timing*; *Measuring D*). It
  blocked neither the code nor the first runs on the rig (2026-11-09), where
  the brain's own share, `sent` − `arrived`, stayed inside its trained band.

## Steps

1. **The camera reader** (O6). The brain spawns `gst-launch-1.0` with the
   Python's pipeline ending in `gdppay ! fdsink fd=1`, parses each 62-byte GDP
   header for the buffer's timestamp, and keeps only the latest frame in
   memory so the pipe never fills. *Check:* Done-when 4: stamps rise frame by
   frame about 16.7 ms apart at 60 frames/s. **Built**, with unit tests for
   the header parse, and run on the rig 2026-11-09: the stamps of the frames
   the brain used rose 33.3 ms apart, two camera frames, in all but 0 to 2 of
   each run's gaps.
2. **The pool** (O2, O4.4, O5). A copy of the synthetic Orin's rules in its own
   module: six entries newest first, the pre-warm, ages from the capture times,
   a failed frame as (−1, −1). *Check:* unit tests copied from the synthetic
   Orin's rules. **Built**, with tests.
3. **The points** (O4.3). Pivot and End through the lens calibration stored in
   `homography.json` (the same iteration OpenCV's `undistortPoints` runs, to
   the same 1e-12 px) and the matrix. *Check:* a still's Pivot at the centre
   stop comes out at the calibration's own centre point. **Built**; unit tests
   check the undo against the forward formula and the matrix against a known
   point. **Checked on the rig** (2026-11-09): with the cart at the centre stop
   and the pole hanging, a run's first frame put Pivot at x = −0.0079 m, against
   the calibration's centre of −0.0075 m, and End 0.4502 m straight below it.
4. **The inputs and the command** (O3, O4.5). The 36 inputs from the pool and
   the last 6 actions; the worker gives a; c = round(a × the top pulse rate
   from the ready line). *Check:* Done-when 2–3, in unit tests. **Built**,
   with tests.
5. **The log** (O7). The folder is made at the Run press; one line per frame,
   written as it happens, so a crash keeps everything before it. *Check:*
   Done-when 5. **Built**.
6. **The worker** (O8). `brain_worker.py`: the engine and the policy loaded
   once, the ready line, then the pipe protocol until its input closes.
   *Check:* its framing has unit tests that need no GPU
   (`test_brain_worker.py`). **Built**, and run on the Orin 2026-11-09.
7. **The brain's service** (O1, O2, O4). Run and idle requests on
   `[brain] address`; the loop of O4 on a thread of its own; the command
   datagram, and the board's replies read after every send. A run request
   while running, or idle while idle, changes nothing. *Check:* Done-when 1,
   on the rig with the cart centred. **Built, and checked on the rig**
   (2026-11-09): two runs ran until STOP, and one ended at the board's FAULT
   when the cart reached a limit switch; the brain was idle after each.
8. **The webserver's routes** (O9, O10). `/run/start` forwards to the brain;
   `/run/stop` forwards idle, then sends the command 0 the set number of
   times; `/run/status` forwards the status; the three camera jobs are refused
   during a run. *Check:* STOP during a run brings the cart to rest and the
   brain to idle. **Built**; on the rig (2026-11-09) `/run/stop` brought the
   brain to idle and sent the zeros each time. Whether the cart came to rest
   was seen at the rig, not logged.
9. **The Run and STOP page** (O9; `/run`, issue 2's plan). A wasm page in its
   own folder, like `__live__/orin/wasm/live`. *Check:* the buttons drive
   steps 7–8. **Built** (2026-11-09), `__live__/orin/wasm/run`: STOP always
   pressable; Run enabled only while the brain is idle and the board answers
   `state` with `"calibrated"`, with issue 7's reason beside it otherwise.
   `/run/status` now adds the board's answer to `state`, asked only while the
   brain is idle. The page reads the status when it opens and after each
   press. **Deployed on the Orin** (2026-11-09): it loaded and showed the
   brain idle and the board calibrated. The buttons have not been pressed on
   the rig yet, so the check is not done.
   **Re-centre** (*The run and idle states*, decided 2026-11-09): a third
   button, enabled when Run is, posting `/calibrate/centre`; STOP also posts
   `/calibrate/stop`, so it ends a Re-centre too; the webserver's `/calibrate`
   moves are refused while the brain reports a run. *Check:* from rest away
   from the centre, Re-centre brings the cart to the centre and the board
   back to `"calibrated"`.
10. **The Arduino Settings page** (O11; blocked on *the settings messages*).
    Its routes send the board the setting and read it back; a set is refused
    while the brain is running. *Check:* Done-when 6, and the value read back
    after a board reset.
11. **The READMEs and DEPLOY.** The brain crate's README says what it does
    and how it is run; `__live__/orin/rust/DEPLOY.md` builds and starts it
    beside the webserver. **Built**: `crates/brain/README.md`, and the deploy
    page's steps 1, 6, 7 and 9 name the brain.

Before Rust work is called done, `cargo clippy --all-targets --no-deps` is
clean in each workspace it touched: the repo root for the pages,
`__live__/orin/rust` for the webserver and the brain.

## References

- `real-loop-dia.md`: the source of truth for this plan.
- `real-arduino-plan.md`: the board that takes the commands and holds
  `runtime_end_margin`.
- `real-pi-plan.md`: `arduino-manager`.
- `synthetic-orin-plan.md`: the synthetic twin, whose pool rules are copied.
- `calibrate-w-homography.md`: the lens, the matrix and the per-frame mapping.
- `__live__/orin/rust/crates/brain/README.md`, `__live__/orin/rust/DEPLOY.md`:
  the brain, and how it reaches the Orin.
