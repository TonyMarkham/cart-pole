# Real Orin Plan: the brain, the webserver and its pages

The trained policy balances nothing until a program on the robot's own computer
does what the synthetic loop did for it: take a camera frame, find the markers,
keep the pool, ask the policy for a command and send it to the Arduino, over and
over until the person says stop. That program is the **brain**, Python on the
Jetson Orin Nano Super. The person drives it from pages on the Orin's Rust
webserver, which also gets an Arduino Settings page for the board's one saved
setting.

Status: **plan** (2026-11-03), written from `real-loop-dia.md` alone, the
source of truth. Where this plan and the diagram disagree, the diagram is right.
Section names in *italics* are the diagram's.

## Done when

1. **Run starts a run and only STOP ends it.** From idle, Run opens the camera
   and starts sending commands; the run goes on through failed frames and
   through the soft limit until STOP; after STOP the brain is idle and sends
   nothing.
2. **The inputs match training's.** For a logged frame, the 36 inputs can be
   rebuilt by hand from the log's coordinates, its capture times and the past
   actions: the pool newest first, ages ÷ 1,000, then the last 6 actions,
   newest first.
3. **The command is the policy's.** For logged inputs, `policy.npz` through
   numpy gives the same a, and c = round(a × 44,600).
4. **The ages are GStreamer's.** Each frame's age comes from its
   `do-timestamp` stamp, not from the time Python read it.
5. **Each run has its log.** `runs/<the Run press's timestamp>/` holds, per
   frame, the model's boxes, the coordinates after the homography and the 36
   inputs.
6. **`runtime_end_margin` is set and read from the Arduino Settings page.**

## Requirements (from the diagram)

The brain (Python, `__live__/orin/python/`):

- **O1: run and idle** (decided 2026-11-03). The brain holds a run/idle state;
  the webserver sends it requests to change state, as it does for
  `calibrate_service.py`. Run starts one run; only STOP ends it. No time limit,
  no episodes, no gap check (*The run and idle states*).
- **O2: at Run** (*The sequence*): open the run's log folder; read the matrix
  (`homography.json`), the lens calibration, `policy.npz` and its run's values;
  pre-warm the pool with six entries of (−1, −1) and the last 6 actions as 0;
  open the camera and drop the settling frames (`[stream] settle_frames`).
- **O3: the trained values from `policy.npz` alone** (*The policy on the
  Orin*): the top pulse rate, the age unit, the past commands and the command
  band, as `policy.py` loads them.
- **O4: each command** (*The sequence*, the loop):
  1. the camera's next frame, 728×544 greyscale, with GStreamer's stamp;
  2. JPEG q90, then decode, as the training frames were made;
  3. detect: per class the model's box, or (−1, −1);
  4. Pivot and End: the lens's bend undone, then through the matrix, (x, z) in
     metres;
  5. push onto the pool at age 0; every older entry ages by the time since the
     last capture;
  6. the 36 inputs; a = tanh(mean), no randomness; c = round(a × 44,600);
  7. send c as one 3-byte datagram to `192.168.0.220:4210`: byte 0 the sign
     (0 negative, 1 positive), bytes 1–2 the size of c as a `u16`,
     little-endian (decided 2026-11-03);
  8. one log line;
  9. wait `[brain] delay_ms`, then take the camera's next frame; frames before
     it are dropped unread (*Timing*).
- **O5: failed frames end nothing.** A frame that does not decode or is not
  728×544 is an entry of (−1, −1) and the run goes on (*The run and idle
  states*).
- **O6: GStreamer's timestamp** (decided 2026-11-03). `nvarguscamerasrc` with
  `do-timestamp`, not Argus's sensor timestamp. `camera.py` must carry it with
  each frame; today it reads raw pixels from `gst-launch-1.0` down a pipe,
  which drops it (*The pool on the rig*).
- **O7: the log** (decided 2026-11-03). One folder per run under `runs` on the
  Orin, named by the timestamp of the Run press; per frame, the raw boxes from
  the detection model, the coordinates after the homography and the array sent
  to the policy (*The log*).

The webserver (Rust, `__live__/orin/rust/crates/webserver`) and its pages
(Rust compiled to WebAssembly, `__live__/orin/wasm`; no hand-written
JavaScript, `AGENTS.md`):

- **O8: Run and STOP.** Run forwards a run request to the brain. STOP sends
  the board the command 0 several times, then asks the brain for idle
  (*The sequence*; *The run and idle states*).
- **O9: the Arduino Settings page** (decided 2026-11-03). It sets
  `runtime_end_margin` on the board and shows the value the board holds
  (*The Uno's settings*). A setting is changed only when no run is going
  (follows).

Rules for both:

- **Tunable values in a toml** (`AGENTS.md`): the brain's in
  `__live__/orin/python/settings.toml` `[brain]` (`delay_ms` today, and the
  runs directory, the service's address, how many times STOP repeats the
  command 0); the webserver's in `__live__/orin/rust/settings.toml`.
- **Live and synthetic share no code** (`AGENTS.md`): the pool's rules are the
  synthetic Orin's (`__synthetic__/orin/python/src/synthetic_orin.py`, the
  rules), copied.

## Not decided by the diagram

Each lands in the diagram first (`real-loop-dia.md`, the *Open* lists):

- **Whether the brain is its own program or part of `calibrate_service.py`**,
  which runs the camera today for the live view, the lens and the measurements
  and keeps them apart. This blocks step 6.
- **Which page carries Run and STOP.** This blocks step 8.
- **The settings messages** to the board and their answers. This blocks step 9.
- **The log's file format, and the timestamp's form in the folder name.** This
  blocks step 5.
- **D**, the delay, not measured (*Timing*; *Measuring D*). It does not block
  the code; it blocks running it on the rig.

## Steps

1. **`camera.py` carries GStreamer's stamp** (O6). Each frame comes with its
   `do-timestamp` time. How the stamp is taken out of GStreamer is the step's
   own work; anything that needs installing waits for the user's permission
   (`AGENTS.md`, *Python policy*). *Check:* Done-when 4: stamps rise frame by
   frame about 16.7 ms apart at 60 frames/s.
2. **The pool** (O2, O4.5, O5). A copy of the synthetic Orin's rules in its own
   module: six entries newest first, the pre-warm, ages from the capture times,
   a failed frame as (−1, −1). *Check:* unit tests copied from the synthetic
   Orin's.
3. **The points** (O4.4). Pivot and End through the live lens calibration and
   matrix, with the code the calibration already uses (`lens.py`, `fit.py`).
   *Check:* a still's Pivot at the centre stop comes out at the calibration's
   own centre point.
4. **The inputs and the command** (O3, O4.6). The 36 inputs from the pool and
   the last 6 actions; `policy.py` gives a; c = round(a × the top pulse rate
   from `policy.npz`). *Check:* Done-when 2–3, in unit tests.
5. **The log** (O7; blocked on *the log's file format*). The folder is made at
   the Run press; one line per frame, written as it happens, so a crash keeps
   everything before it. *Check:* Done-when 5.
6. **The brain's service** (O1, O2, O4, O8; blocked on *its own program or part
   of `calibrate_service.py`*). Run and idle requests on the Orin's own
   address, the same way as `calibrate_service.py`'s `[service]`; the loop of
   O4; the command datagram. A run request while running, or idle while idle,
   changes nothing. *Check:* Done-when 1, on the rig with the cart centred.
7. **The webserver's routes** (O8). Run forwards to the brain as `live.rs`
   forwards to the calibration service; STOP sends the command 0 the set number
   of times, then forwards idle; a status route reports run or idle. *Check:*
   STOP during a run brings the cart to rest and the brain to idle.
8. **The Run and STOP page** (O8; blocked on *which page*). A wasm page or
   part of one, like `__live__/orin/wasm/live`. *Check:* the buttons drive
   steps 6–7.
9. **The Arduino Settings page** (O9; blocked on *the settings messages*). Its
   routes send the board the setting and read it back; a set is refused while
   the brain is running. *Check:* Done-when 6, and the value read back after a
   board reset.
10. **The READMEs.** `__live__/orin/python/README.md` and
    `__live__/orin/rust/README.md` gain the brain, its settings, the routes and
    the pages, written for a layman.

Before Rust work is called done, `cargo clippy --all-targets --no-deps` is
clean in each workspace it touched: the repo root for the pages,
`__live__/orin/rust` for the webserver.

## References

- `real-loop-dia.md`: the source of truth for this plan.
- `real-arduino-plan.md`: the board that takes the commands and holds
  `runtime_end_margin`.
- `real-pi-plan.md`: `arduino-manager`.
- `synthetic-orin-plan.md`: the synthetic twin, whose pool rules are copied.
- `calibrate-w-homography.md`: the lens, the matrix and the per-frame mapping.
- `__live__/orin/python/README.md`, `__live__/orin/rust/README.md`: the Orin's
  code today.
