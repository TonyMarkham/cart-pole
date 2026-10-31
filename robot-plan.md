# Robot Plan — the real Orin and the real Arduino

A policy trained in the synthetic loop is only worth having if the real
robot does what the loop's stand-ins did, so this plan builds the two
real computers: the **Orin** (the Jetson Orin Nano Super, the robot's
computer: camera, detection, the policy) and the **Arduino** (the rig's
UNO R4 WiFi, which turns each command into pulses for the DM542 stepper
driver). Each is one source with its synthetic twin, built two ways;
only the edges differ, where frames come from and where commands go
(`AGENTS.md`, *The real robot*).

Status: **plan, being written** (started 2026-10-29). Not built.

**What still stops a live run when this plan is built:** a trained
policy that balances the pole (training is under way), and the real
delay D, the time from a capture to its command reaching the Arduino
(detection + the policy's inference + the WiFi hop), which is not
measured. The policy trains on 8–15 ms, detection alone
(`synthetic-loop-dia.md`, *Contracts*). It also trains on one command
every 33.3–50 ms, 20–30 Hz (`synthetic-loop-dia.md`, *Nothing happens
between commands*): in the synthetic loop the orchestrator times that, and
it has no real counterpart, so the real Orin must run that timer itself,
and fill its pool only from the frames it acts on.

**Not decided:** what this plan is written from. The rule is diagram →
plans → code, and `synthetic-loop-dia.md` draws only the synthetic loop.
Proposed (2026-10-29), not answered: a short `robot-dia.md`, the
sequence of one live run, first.

## Terms

- **DIR:** the DM542's direction input. Its level (high or low) sets
  which way the motor turns; which way the cart then goes on the rail
  depends on the motor's wiring.
- **A positive command:** a pulse rate above 0. In MuJoCo it moves the
  cart towards `+x`, which is right in the camera's frame
  (`mujoco-plan.md`, *The sign convention*), so on the rig a positive
  command must move the cart right as the camera sees it.
- **The page:** the web page the Arduino serves over WiFi, the pattern
  of `calibrate` (`calibrate/README.md`, *The page*): buttons that each
  send a POST, so loading the page never moves the cart, and everything
  the board knows.

## The Arduino's page (user, 2026-10-29)

The live firmware serves a page in the `calibrate` pattern with:

- **STOP** — stops the cart.
- **Calibrate** — homes and centres the cart, as `calibrate` does: slow
  to either limit switch, slow to the other counting pulses, fast back
  half the span.
- **Run** — the board takes the Orin's commands and steps by them.
- **A Left / Right toggle** — which way DIR high moves the cart, in the
  camera's frame. This answers *DIR's level*
  (`synthetic-arduino-plan.md`, *Not decided by the diagram*) at run
  time instead of in the build: the board drives DIR high for a
  positive command when the toggle says Right, low when it says Left.
  To set it: watch which way the cart goes and flip the toggle until a
  positive command moves it right.

Follows from rules already set:

- **The toggle's power-up value is a setting** in the firmware's toml,
  compiled in as `calibrate.toml`'s values are (`AGENTS.md`: tunable
  values live in a toml).
- **Run needs a finished calibration**, as `calibrate`'s Go to centre
  does: the travel limit counts pulses from the centre.

**The health check** (user, 2026-10-29): a route on the board's web
server that only answers "I'm alive", so the Orin can check it reaches
the Arduino (*The Orin's page*). A health check is the common web
pattern for this: a fixed path that a running server always answers,
with nothing done and nothing changed. It moves nothing, so it is a
plain GET, unlike the buttons' POSTs. It is enabled only while the
robot is not running: during Run the board does not answer it.
What it checks: that the board is on the network and its web server
answers. It does not check the UDP port the commands arrive on, which
is a different socket. Proposed, not decided: the path `/health`.

Proposed, not decided:

- **The toggle is ignored while the cart moves** (as `calibrate`
  ignores buttons during a move): flipping DIR mid-run would reverse
  the cart at once, past the acceleration ceiling.

Open:

- **STOP during Run.** `calibrate` reads the page only by pausing
  between pulses, because its pulses are timed by the processor, so a
  STOP in a fast move waits until the move ends (`calibrate/README.md`,
  *What STOP cannot stop*). Run is fast moves all the time. If Run's
  pulses come from the RA4M1's GPT timer in hardware, as
  `synthetic-arduino-plan.md` (*The board and the top rate*) intends,
  the processor can read the page without pausing them; not checked on
  the board. The commands arrive by the bridge's UDP push mode (the
  user's fork, `v0.6.0-interrupt.2`: each datagram is sent to the RA4M1
  unasked as `+UDPRX: <sock>,<len>|<bytes>`); the page is TCP, which
  push mode does not cover. Which bridge build is on the rig is not
  recorded.

## The Orin's page (user, 2026-10-29)

The Orin hosts a website, served by Axum (a Rust web-server library),
with:

- **A switch between the two modes**, `WebSocketServer` and
  `RobotBrain` (*The Orin's modes*).
- **Validate Arduino Connection** — a button that calls the Arduino's
  health check and shows whether the Arduino answered.

Follows from what is set:

- **The website and the detector are two programs.** Axum is Rust;
  detection on the robot is Python (`AGENTS.md`). How the website
  switches the Python program's mode (restart it in the other mode, or
  tell it over a local socket) is not set.

Proposed, not decided:

- **`config.toml`'s mode is the mode at start-up**, and the website
  switches it while running, as the Arduino's Left / Right toggle has
  its power-up value in a toml.

Follows from rules already set:

- **The Arduino's address and how long to wait for its answer are
  settings** in the Orin's toml (`orin/settings.toml` holds the
  detector's today).

## The Orin's modes (user, 2026-10-29)

The Orin's program has a `config.toml` that says which job it runs:

- **`WebSocketServer`** — the proof-of-concept slices, starting with
  Slice 1: serve the viewer stills and their detections.
- **`RobotBrain`** — the robot's real job: detect, keep the pool, run
  the policy, send the commands to the Arduino (later slices).

`orin/settings.toml` (the detector's settings) is a separate file,
beside it.

## Slice 1 — the Orin sees the markers (user, 2026-10-29)

The first thing the real Orin has to prove is that it can find the four
markers in its own camera's frames, because everything after it (the
pool, the policy, the commands) is built on those detections. This
slice is that and nothing more: capture a still, detect the markers, and
send both to a viewer app on a Windows PC, because the Orin has no
monitor. "Naive" means no pool, no timing, no policy and no Arduino.

**What it reads and writes:** reads the IMX296 camera; for each still,
works out each marker's box and sends the still and the boxes to the
viewer.

**The shape** (user, 2026-10-29):

- **The Orin is the server; the Windows viewer is the client.** They
  talk over one WebSocket (a two-way connection that carries whole
  messages, text or binary) on the local network, 192.168.0.x.
- **The Orin takes a still, detects the markers, and sends the
  picture and the detection boxes** through the WebSocket.
- **The viewer draws the picture with the boxes on it.**

**The parts:**

1. **Capture.** One frame from the camera at its native 1456×1088.
   How the camera is read on the Orin is not recorded: `AGENTS.md`
   says only that it is validated for default colour captures.
2. **Into the training frames' look.** The detector learned on
   728×544 frames of BT.601 luma (the standard weighting of red, green
   and blue into one brightness) in three equal channels, saved as
   JPEG quality 90 (`synthetic-loop-dia.md`, *Contracts*, the frame).
   The real frame is made the same: halved to 728×544, turned to luma,
   JPEG-encoded at q90. Encoding to JPEG and decoding again costs time,
   but it hands `orin/src/detector.py` the same input the synthetic
   Orin gets, so the detection code is the loop's, unchanged.
3. **Detect.** `Detector.detect(jpeg)` in `orin/src/detector.py`, on
   an FP16 TensorRT engine (the model compiled for one GPU, at half
   precision) built on the Orin by `orin/src/build_engine.py` from
   `training/runs/cp2-640-base/weights/best.onnx`. An engine works only
   on the GPU and TensorRT version that built it, so the Orin builds its
   own (`synthetic-orin-plan.md`, R2).
4. **Send.** Over the WebSocket, the still and its detections: per
   class, the box (centre and size, normalized to [0, 1] of the frame,
   so the viewer can draw at any size), its confidence (the detector's
   score, 0–1, for that box), and the time detection took.
5. **The viewer** (Windows): connects to the Orin, draws each still
   with its boxes, class names and confidences.

Follows from what is set:

- **The boxes need the detector to return their size.** The marker
  frame carries only each box's centre (`synthetic-loop-dia.md`,
  *Contracts*); the detector has the size in its output rows but drops
  it. Returning it beside the centre changes nothing the loop uses.
- **The Orin's side is Python**, in `orin/`, beside the detector
  (`AGENTS.md`: detection on the robot is Python); its `websockets`
  library serves as well as it connects.
- **The addresses are settings:** the Orin's listening port in
  `orin/settings.toml`, the Orin's address in the viewer's toml.

Proposed, not decided:

- **The still sent is the 728×544 JPEG the detector read**, so the
  viewer shows exactly what the detector saw, and it is already
  encoded.
- **One message pair per still**, as the loop's frames go
  (`synthetic-orin-plan.md`, *Messages*): a text message with the id
  and the detections, then a binary message with the JPEG.
- **The viewer is Rust with GPUI**, like `pi-control`, the desktop
  companion in `D:\git\webrtc-pi\app` that already shows the Pi's
  JPEG preview.

**Done when:** with the rig in front of the camera, the viewer shows the
stills with a box on each of the four markers, and one saved still's
boxes can be checked by hand against the image.

**Settings** (in `orin/settings.toml`): the camera's, the WebSocket's
port, and where saved stills go.

**Not known yet; each blocks a part:**

- **How the camera is read on the Orin** (part 1). The Pi's tools
  (`rpicam-still`, used for the real datasets, `DATASETS.md`) do not
  run on a Jetson.
- **How the halving is done** (part 2): the filter that turns
  1456×1088 into 728×544. Unity renders 728×544 directly
  (`unity-player-plan.md`, step 4), so there is no training-side method
  to copy; the placeholder *The real camera's greyscale* in
  `unity-player-plan.md` asks for the same.
- **How the `orin/` uv project gets TensorRT on the Orin** (part 3).
  It names NVIDIA's `tensorrt-cu12` pip wheels today
  (`orin/pyproject.toml`); on the Orin TensorRT 10.3 comes with JetPack
  6.2.3. The 2026-10-23 test on the Orin ran Python with TensorRT
  (`detector-plan.md`, *Results*), but how it was installed is not
  recorded. Installing anything needs the user's permission.

## To cover (from the records, not decided)

- **Arduino:** the `arduino` crate's `live` build (planned in
  `arduino/src/lib.rs`, not built); receiving the 4-byte command over
  UDP; STEP from `Stepper` (`unor4-hal` needs a run-time rate change and
  a pulse count); the DM542's ~5 µs STEP and DIR timing (not approved).
- **Orin:** camera capture to the training frames' domain (728×544,
  BT.601 luma in three equal channels, JPEG q90; the open greyscale
  issue in `AGENTS.md`); the pool's ages from the camera's timestamps
  instead of MuJoCo's clock; the policy on the Orin (how it gets there
  is open, `trainer-plan.md`); the last 6 commands in its input; the
  command sent over UDP.
- **Both:** an episode's start (homing, the pole hanging at rest) and
  end (no MuJoCo sites on the rig; the board's travel limit is the only
  stop); measuring D.

## References

- `synthetic-arduino-plan.md`, `synthetic-orin-plan.md` — the synthetic
  twins
- `calibrate/README.md` — the page pattern, homing
- `control-design.md` — the wiring
- `D:\git\unor4-wifi-interrupt-bridge`, branch `feature/interrupt`,
  `docs/releases/v0.6.0-interrupt.2.md` — UDP push mode
