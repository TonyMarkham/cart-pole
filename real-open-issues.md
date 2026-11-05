# Real loop: the open issues, researched

The real loop can't be built or run safely while questions in its design are
unanswered. Issues 1–12 are listed as **Open** in `real-loop-dia.md` and repeated
in the plan that depends on it. Issues 13–15 are things it says are not yet
checked, and 16–17 were found while researching. This file takes every open issue in turn and sets
down what the repo, the measurements and the vendors' documents already say
about it, so the person deciding has the facts in one place.

Status: **research** (2026-11-04), not a plan. Nothing here decides anything.
Each issue stays open until the user decides it, and the decision lands in
`real-loop-dia.md` first (that file, *This diagram is the single source of truth
for the real loop*).

Each statement is marked, as the diagram marks its own:

- **Decided:** the user's decision, with its date and where it is recorded.
- **Measured:** a number taken on the rig or the Orin, with its record.
- **Reported:** a fact relayed from another session or source and not checked
  here. Most of these come from `HANDOFF.md`, *The HAL's step generator*: the
  HAL is `unor4-hal`, the library that drives the board's hardware.
- **Follows:** worked out from the above, with the working shown.
- **Proposed:** a suggestion from the agent. The user has not decided it.
- **Not known:** nothing in the repo or the sources answers it.

Line numbers point into the files as they stand on 2026-11-04.

**Contents**

- *Timing and the delay D:* 1
- *The brain and the webserver:* 2, 3
- *The board and the commands:* 4, 5, 6, 7, 8
- *The Uno's settings:* 9, 10, 11
- *The log:* 12
- *Not yet checked on the rig:* 13, 14, 15
- *Found while researching, in no Open list:* 16, 17

---

> ## 1. D, the delay: not measured, and no way to measure it chosen
>
> > [!WARNING]
> > **Worse than an open issue: no decision can answer it, only an experiment.**
> > D has to be measured on the rig, and no way to measure it has been chosen
> > (*How D could be measured*, below). Until then nobody knows whether the
> > trained policy fits the rig at all. The measurements so far point to D
> > being past the trained 8–15 ms (*What the sources say*, below), and if it
> > is, the policy is trained again (`real-loop-dia.md` lines 107–109).
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *Timing*, line 107: "**Open:** D is not measured".
> > - `real-loop-dia.md`, *Measuring D*, lines 504–515: "**Open.**"
> > - `real-loop-dia.md`, *What exists today*, line 576: "D measured | not done".
> > - `real-orin-plan.md`, *Not decided by the diagram*, lines 109–110.
> > - `robot-plan.md`, lines 14–23: D "is not measured".
> > - `HANDOFF.md`, *Still open*, line 249.
>
> **What D is:** the time from the moment a frame is exposed until its command
> reaches the Arduino. The policy only knows the world as it was D ago, so it
> must have trained with the D the rig really has.
>
> **What the sources say:**
>
> - **Decided:** the policy trained with D drawn between 8 and 15 ms
>   (`__synthetic__/trainer/python/settings.toml`, `rnd_delay_min` and
>   `rnd_delay_max`, lines 71–72). That file's own comment, lines 68–70, says the
>   band is "the detection time measured on the Orin" and that the policy's
>   inference and the WiFi hop "are not in it yet".
> - **Measured:** the Orin's whole Python detection path, decoding included,
>   took 11.9–14.5 ms (`detector-plan.md`, *Results*, quoted in
>   `real-loop-dia.md` line 105).
> - **Measured:** the WiFi round trip to the board, using the board's `pins`
>   answer, 1,000 times, on 2026-11-02: median 33.0 ms, 90% within 41.1 ms, 99%
>   within 112.5 ms, 4.2% lost (`__live__/orin/python/README.md`, *The WiFi round
>   trip to the Arduino*). Pinging the board's WiFi chip, which answers without
>   the board's processor, took 3.5–125.0 ms, an average of 66.4 ms. Pinging the
>   router took 1.1–2.4 ms.
> - **Follows:** detection alone already reaches 14.5 ms against the band's top
>   of 15 ms, leaving 0.5 ms. Everything else in D comes on top of it: getting
>   the frame from the camera, the policy, and the WiFi hop. A round trip is
>   not a one-way hop, but every measurement of the radio path is well over
>   0.5 ms. So D is past the trained band unless the hop turns out far shorter
>   than any measurement so far suggests.
> - **Decided:** if D lies outside 8–15 ms, the band in `settings.toml` is set to
>   the measured one and the policy is trained again (`real-loop-dia.md` lines
>   107–109).
> - **Decided, a future feature, not now** (user, 2026-11-02): turn off the WiFi
>   chip's power saving and measure again (`real-loop-dia.md` lines 523–529). The
>   README calls power saving the likely cause of the slow and scattered pings,
>   and says this is not proven.
>
> **How D could be measured, and what stands in the way.** The Orin's clock can
> time the frame's arrival, detection, the policy and the moment the datagram
> leaves. It can't time the exposure up to the frame's arrival, the WiFi hop, or
> the bridge's hop to the board's processor (`real-loop-dia.md` lines 506–515):
>
> - **Decided:** the brain uses GStreamer's timestamp (`nvarguscamerasrc` with
>   `do-timestamp`), not Argus's sensor timestamp (user, 2026-11-03,
>   `real-loop-dia.md` lines 286–291). GStreamer is the program that chains the
>   camera's video steps together, and Argus is NVIDIA's camera software beneath
>   it. The decided stamp is the right one for the pool's ages. It doesn't
>   measure D, which starts at exposure.
> - **From GStreamer's documentation:** `do-timestamp` makes a source stamp each
>   buffer with the pipeline's running time when it is sent on, and is meant
>   for live sources (GstBaseSrc, `set_do_timestamp`;
>   <https://developer-old.gnome.org/gstreamer-libs/stable/GstBaseSrc.html>).
>   So the stamp is taken after the exposure, the sensor's readout and the
>   transfer, as the diagram says (line 289: "not the moment of exposure").
> - **From NVIDIA's developer forum, not official documentation:** the sensor's
>   own timestamp comes from the capture interrupt and is read through Argus's
>   capture metadata (`ICaptureMetadata::getSensorTimestamp()`), not from a
>   GStreamer pipeline. One user who changed the plugin to read it found it about
>   35 ms away from the buffer's time. That thread has no answer
>   (<https://forums.developer.nvidia.com/t/nvarguscamerasrc-timestamping/199563>,
>   <https://forums.developer.nvidia.com/t/how-to-get-timestamp-using-nvarguscamerasrc/167865>).
>   Not checked against NVIDIA's own documentation.
> - **Built, but carries no stamp:** today `camera.py` runs `gst-launch-1.0` and
>   reads raw pixels from a pipe (`__live__/orin/python/src/camera.py` lines
>   42–63), so it carries no timestamp at all. `real-orin-plan.md` step 1 is the
>   work to carry GStreamer's stamp.
> - **The board's side of the hop:** a command gets no answer
>   (`real-loop-dia.md`, the command contract, line 553: "no id"). The only
>   answer the board gives today, `pins`, costs it 5 exchanges with its WiFi chip
>   (`__live__/orin/python/README.md` lines 300–302). So that answer measures
>   more than a command's hop.
>
> **What it would take:**
>
> - **Not known:** how long a frame takes from exposure to the moment GStreamer
>   stamps it, on this camera and this Orin.
>
> > ### RESOLUTION PLAN
> >
> > **Measure D in two parts, then compare it with the trained band:**
> >
> > 1. **The Orin's part, every run:** the brain logs, on the Orin's own clock,
> >    when each frame arrives and when its datagram leaves (`real-loop-dia.md`
> >    lines 506–507). This adds two fields, `arrived` and `sent`, to issue 12's
> >    log line.
> > 2. **The whole of D, once, optically:** the board toggles its LED (D13,
> >    `calibrate/README.md` line 53) each time a command arrives, which is one
> >    pin write, a fixed rule per command. With the LED in the camera's view,
> >    the frame that first shows a toggle, minus the frame whose command caused
> >    it, is D plus the wait for the next exposure: 0 to 16.7 ms, half a frame
> >    on average. Over many commands, the mean D is the mean measured less 8.3
> >    ms.
> > 3. **If D lies outside 8–15 ms,** `rnd_delay_min` and `rnd_delay_max` in
> >    `__synthetic__/trainer/python/settings.toml` become the measured band, and
> >    the policy is trained again (decided, `real-loop-dia.md` lines 107–109).
> > 4. `real-loop-dia.md` replaces its two Open entries (line 107 and lines
> >    504–515) with the method and the result.

---

> ## 2. Which page carries Run and STOP
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The run and idle states*, lines 193–195.
> > - `real-loop-dia.md`, *What exists today*, line 571: "which page is open".
> > - `real-orin-plan.md`, *Not decided by the diagram*, line 105; it blocks
> >   step 8, line 142.
> > - `HANDOFF.md`, *Still open*, line 245.
>
> **What the sources say (the pages that exist, `__live__/orin/rust/README.md`,
> *Crates* and *Server routes*):**
>
> | Page | What it does today |
> |---|---|
> | `/calibrate` | The seeks, the move to the centre, a full-width STOP, and the measuring window |
> | `/live` | The camera's picture with the detector's boxes, for aiming the camera |
> | `/arduino` | The board's pins, read about once a second |
> | `/lens` | The lens measurement |
> | `/viewer` | Shows a pushed picture |
>
> - **Decided:** the calibration's seeks and the move to the centre come before
>   every session (`real-loop-dia.md` lines 226–229). Today they run from
>   `/calibrate`.
> - **Follows:** the STOP on `/calibrate` can't stop a run. It sends the text
>   `Stop` (`__live__/orin/rust/README.md` line 101), which ends only a seek or a
>   move to the centre and is ignored otherwise
>   (`__live__/arduino/rust/crates/calibration/README.md` lines 72–82). A run's
>   STOP sends the command 0 instead (`real-loop-dia.md` lines 357–362). So
>   wherever Run's STOP goes, it is a different button from today's, or a
>   STOP that sends both.
> - **Follows:** the webserver holds one move at a time and answers
>   `/arduino/pins` with 503 during a move (`__live__/orin/rust/README.md` lines
>   97–99). During a run a seek or a centring must be refused the same way. A
>   board moving under the commands also throws away other datagrams, as it
>   does during a seek (firmware README lines 67–70), unless its command
>   handling is built otherwise.
>
> > ### RESOLUTION PLAN
> >
> > **Decided** (user, 2026-11-04): Run and STOP are on a new page, `/run`, on
> > the Orin's webserver.
> >
> > 1. `real-loop-dia.md` records the decision in place of the Open entry
> >    (*The run and idle states*, line 195). It also names `/run` where the
> >    sequence's Run arrow says "which page is open" (line 133) and in *What
> >    exists today* (line 571).
> > 2. `real-orin-plan.md` drops the entry from *Not decided by the diagram*
> >    (line 105). Step 8 becomes the `/run` page: a WebAssembly page in its own
> >    folder under `__live__/orin/wasm/`, as `live` and `lens` are.
> > 3. The webserver serves `/run` and gets the Run, STOP and status routes of
> >    `real-orin-plan.md` step 7.
> > 4. `/run`'s STOP sends the board the command 0, not the text `Stop`, which
> >    the board ignores outside a seek or a move to the centre (*What the
> >    sources say*, above).
> > 5. While a run is going, the webserver refuses the `/calibrate` moves, as it
> >    already refuses a second move.

---

> ## 3. Whether the brain is its own program or part of `calibrate_service.py`
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The run and idle states*, lines 196–199.
> > - `real-orin-plan.md`, *Not decided by the diagram*, lines 102–104; it blocks
> >   step 6, line 133.
> > - `HANDOFF.md`, *Still open*, lines 245–246.
>
> **What the sources say:**
>
> - **Built:** the service runs one job at a time on one thread, which owns the
>   camera and the GPU. The jobs are a measurement, the live view and the lens
>   view (`MEASURE, LIVE, LENS`, `__live__/orin/python/src/calibrate_service.py`
>   line 47; `Service`, lines 50–53; `run`, lines 487–517). It loads the
>   TensorRT engine once at start (lines 488–492). It answers HTTP from other
>   threads (`ThreadingHTTPServer`, line 24), and everything those threads read
>   is behind one lock.
> - **Built:** the camera is opened only for the length of a job, so other
>   programs (`capture_push.py`, `live_detect.py`) can use it between jobs
>   (`__live__/orin/python/README.md` lines 78–79). Nothing stops another
>   program from trying to open it during a job: the lock keeps only the
>   service's own jobs apart. The service does answer `GET /busy`
>   (`calibrate_service.py` lines 729–730).
> - **Built:** the webserver already forwards to the service on the Orin's own
>   address (`[service]`, `127.0.0.1:8090`, `__live__/orin/python/settings.toml`
>   lines 101–105; `live.rs`).
> - **Follows, for a part of the service:** a run becomes a fourth job kind. It
>   gets the camera's mutual exclusion and the loaded engine for free, and is
>   forwarded to the same way as `/live`.
> - **Follows, for its own program:** it needs its own way to keep the camera
>   from the service, such as asking `/busy` or the webserver's one permit, and
>   it loads its own copy of the engine.
> - **Not known:** whether the service's HTTP threads, run beside a loop in the
>   same Python process, disturb the loop's timing. Python runs one thread at a
>   time (its global lock). Not measured. Nor is it known whether two copies of
>   the engine fit in the Orin's memory together.
>
> > ### RESOLUTION PLAN
> >
> > **Decided** (user, 2026-11-04): the brain is its own program, a Rust
> > orchestrator. It calls Python only for what has to be Python or is already
> > built there: finding the markers (detection on the robot is Python,
> > `AGENTS.md`, user 2026-10-23) and the policy's inference (`policy.py`,
> > built and checked against Stable-Baselines3). The split of each event
> > between Rust and Python is the agent's.
> >
> > ```mermaid
> > sequenceDiagram
> >     autonumber
> >     participant U as Person (browser, /run)
> >     participant W as Webserver (Rust)
> >     participant B as Brain (Rust)
> >     box transparent Python
> >         participant D as Detector
> >         participant Pol as Policy
> >     end
> >     participant C as Camera
> >     participant A as Arduino
> >
> >     U->>W: Run
> >     W->>B: run
> >     B->>B: idle → run
> >     B->>B: open the run's log folder in runs/, named by the Run press's timestamp
> >     B->>B: read homography.json: the homography, and the lens calibration it was fitted with, halved for 728×544
> >     B->>B: check homography.json's "ok", as the Orin's Python README requires
> >     Note over B: What the brain does when "ok" is false is not stated in any source
> >     B->>Pol: load policy.npz
> >     Pol-->>B: the run's values: top pulse rate, age unit, past commands, command band
> >     B->>B: pre-warm the pool: six entries with no detection, (−1, −1), at gaps before the first frame
> >     B->>B: set the last 6 actions to 0
> >     B->>C: open the camera
> >     C-->>B: the settling frames, dropped
> >     Note over D: When the TensorRT engine loads is not in real-loop-dia.md
> >
> >     loop every command, until STOP
> >         C-->>B: the camera's next frame, 1456×1088 halved to 728×544 greyscale, with its GStreamer timestamp
> >         B->>D: the frame
> >         D->>D: encode the frame as a JPEG, quality 90
> >         D->>D: decode the JPEG
> >         D->>D: detect: per class, the model's box
> >         D-->>B: per class, the box's centre and size normalised to [0, 1] and its confidence, or (−1, −1) for a marker not found
> >         B->>B: scale Pivot's and End's centres to pixels of the 728×544 frame: x × 728, y × 544
> >         B->>B: undo the lens's bend on those pixels
> >         B->>B: the homography: Pivot's and End's (x, z) in metres
> >         B->>B: the new entry's 5 numbers: age 0, Pivot's (x, z), End's (x, z), a marker not found kept as (−1, −1)
> >         B->>B: age every older entry by the time since the last capture
> >         B->>B: push the new entry at the front, the pool being newest first
> >         B->>B: pop the oldest entry
> >         B->>B: the 36 inputs: the pool's 30 numbers, ages ÷ 1,000, then the last 6 actions, newest first
> >         B->>Pol: the 36 inputs
> >         Pol-->>B: a = tanh(mean), no randomness
> >         B->>B: c = round(a × 44,600)
> >         B->>B: a becomes the newest of the last 6 actions
> >         B->>A: c as 3 bytes: the sign, then the size as a u16, little-endian
> >         B->>B: write the log line: the model's boxes, the coordinates after the homography, the 36 inputs
> >         B->>B: wait delay_ms
> >         C-->>B: the frames that arrived during the wait, dropped unread
> >     end
> >
> >     U->>W: STOP
> >     W->>B: idle
> >     W->>A: the command 0, several times
> >     B->>C: close the camera
> >     B->>B: run → idle
> >     B->>B: close the log folder
> >     B-->>W: idle
> >     W-->>U: shows it
> > ```
> >
> > 1. `real-loop-dia.md` records the decision in place of the Open entry
> >    (*The run and idle states*, lines 196–199). Its sequence's box "The
> >    Orin's brain (Python, not built)" (line 118) becomes the Rust brain, with
> >    the Python detector and policy beside it.
> > 2. `real-orin-plan.md` drops the entry from *Not decided by the diagram*
> >    (lines 102–104), and its requirements O1–O7 and steps 1–6, written for a
> >    Python brain in `__live__/orin/python/`, are rewritten for the Rust one.
> > 3. The brain is a new crate in `__live__/orin/rust/crates/`, the Orin's own
> >    workspace, built on the Orin (`AGENTS.md`, *Repository map*, `__live__/`).
> >
> > Still open within this plan, each to land in `real-loop-dia.md` first:
> >
> > - which Python process finds the markers and runs the policy,
> >   `calibrate_service.py` or a new one;
> > - how the brain and that process talk each frame, and what that adds to D;
> > - what keeps the brain and `calibrate_service.py` from opening the camera
> >   together;
> > - when the TensorRT engine loads;
> > - what the brain does when `homography.json` says `"ok": false`.

---

> ## 4. DIR's level for a positive command
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The command and the Arduino's Run*, lines 427–431.
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 88.
> > - `synthetic-arduino-plan.md`, *Not decided by the diagram*, line 81:
> >   "**DIR's level** — which sign is the high level on the DIR pin."
> > - `__synthetic__/orchestrator/rust/crates/arduino/src/signals.rs`, lines
> >   1–4: "Which pin level is which is not decided yet".
> > - `robot-plan.md`, *Terms*, lines 32–38, and *The Arduino's page*, lines
> >   53–59 (the Left / Right toggle).
> > - `control-design.md`, *The driver's pins*, lines 143–145: "**DIR's level**
> >   stays undecided".
> > - `__live__/arduino/rust/crates/calibration/calibration.toml`, lines 71–74:
> >   "NOT yet checked on the rig".
> > - `__live__/arduino/rust/crates/calibration/README.md`, *The moves*, lines
> >   125–130.
> > - `__live__/arduino/rust/crates/calibration/src/direction.rs`, lines 14–17.
> > - `HANDOFF.md`, line 204 ("DIR forward | **open**") and *Still open*, lines
> >   234–235.
>
> **What it means:** DIR is the stepper driver's direction input, set by the
> Arduino's pin D5. The question is which level on D5 a positive command needs.
> A positive command must move the cart to the right as the camera sees it
> (`robot-plan.md`, *Terms*, lines 35–38), because that is the direction the
> policy learned as `+x`.
>
> **What the sources say:**
>
> - **Positive goes towards the motor's end.** The trainer gave DIR no level,
>   only positive and negative
>   (`__synthetic__/orchestrator/rust/crates/arduino/src/signals.rs` lines 1–4).
>   Positive is `+x` (`mujoco-plan.md` lines 281–284), and in the MJCF the
>   trainer loads, `+x` is the motor's end: `asy-base-motor` is at x = +0.48
>   and `asy-base-idler` at x = −0.485
>   (`unity/projects/cart-pole/Assets/Simulation/Robot/asy-cart-pole.step~/asy-cart-pole.xml`
>   lines 352 and 562). The rig's camera shows the motor's end on the right
>   (measured 2026-11-01, `calibrate-w-homography.md` lines 139–141).
> - **2026-11-01, today's `calibration` firmware:** the idler's seek runs with
>   D5 high (`calibration.toml` lines 75–81), and that day it ended at the
>   idler's end (same measurement). That means D5 high goes to the idler, so
>   positive is D5 low, provided the rows then were the rows now. No document
>   records that they were.
> - **2026-10-28, the older `calibrate` firmware:** D5 low drove the cart to
>   D3, its first end (`calibrate/README.md` lines 8–9; `jump-limit-results.md`
>   lines 41–42). Today's `calibration.toml` puts D3 at the idler's end. If D3
>   was the idler's end then too, D5 low goes to the idler, so positive is D5
>   high.
> - **The two records contradict each other,** and no document records a
>   wiring or settings change between them.
> - **Built:** today positive rates run with D5 high (`Direction::FORWARD`,
>   `__live__/arduino/rust/crates/calibration/src/direction.rs` line 18).
> - **Decided** (user, 2026-10-29, `robot-plan.md` lines 52–59): a Left / Right
>   toggle sets it at run time. The page it was on, the board's own, no longer
>   exists (the firmware's README, line 290).
>
> > ### RESOLUTION PLAN
> >
> > > [!IMPORTANT]
> > > **UNRESOLVED**
> >
> > To settle it: on the rig, send `CalibrateIdlerTravelLimit`, which runs with
> > D5 high, and watch which way the cart goes. If it goes to the camera's left,
> > positive is D5 low. If it goes right, positive is D5 high. Then:
> >
> > 1. `calibration.toml` gains the D5 level for a positive command, compiled
> >    in by `build.rs`, and `Direction::FORWARD` takes it (`direction.rs` line
> >    18). It changes with the wiring, so it belongs in a toml (`AGENTS.md`,
> >    *Tunable values live in a toml*; `robot-plan.md` lines 63–65).
> > 2. `real-loop-dia.md` records it in place of the Open entry (lines 427–431).
> > 3. `real-arduino-plan.md` drops the entry (line 88).

---

> ## 5. What the board does when commands stop coming
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The command and the Arduino's Run*, lines 432–434:
> >   "Today's rule would hold the last command until the soft limit brought the
> >   cart to rest at a stop point."
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 89.
> > - `HANDOFF.md`, *Still open*, line 236.
>
> **What the sources say:**
>
> - **Between commands the board holds the last one,** in training and on the
>   rig alike: the command is "the last one received"
>   (`__synthetic__/orchestrator/rust/crates/arduino/src/ramp.rs` lines 8–9).
> - **Training never lost a command.** The synthetic Arduino gets each command
>   "by call instead of radio" (that crate's `lib.rs` lines 11–13). The policy
>   trained with 33.3–50 ms between commands (`real-loop-dia.md` lines 70–72).
> - **Measured:** at most 4.2% of datagrams are lost. That is the share of
>   round trips with no answer within 500 ms, and a round trip can lose either
>   leg (`__live__/orin/python/README.md` line 315).
> - **Follows, the common case:** up to about 1 command in 24 is lost. Each loss
>   doubles that gap, to 67–100 ms at the trained rates, longer than any gap the
>   policy has seen.
> - **Follows, the rare case** (the brain or the Orin crashes, or the WiFi
>   drops): the last command holds until the soft limit brings the cart to rest
>   at an end's stop point (`real-loop-dia.md` lines 432–434). From the centre at
>   top speed that takes about 29,910 ÷ 44,600 ≈ 0.67 s, and the pole falls. If
>   the Orin is down, the webserver's STOP is down with it, and the power switch
>   is the only stop (`real-loop-dia.md` line 157).
> - **Built:** while the cart moves, the firmware already wakes at every 2 ms
>   tick (`__live__/arduino/rust/crates/calibration/src/pins.rs` lines 220 and
>   239). A rule that counts those ticks needs no new wake-up. At rest there is
>   nothing to stop.
> - **Decided:** every Uno rule is a fixed rule per tick or per command
>   (`AGENTS.md`, *The real robot*, user 2026-10-28).
>
> > ### RESOLUTION PLAN
> >
> > **Resolved by an existing decision:** "Orin is the brain, uno is the dumb
> > muscle" (user, 2026-11-02, `real-loop-dia.md` line 358). The board holds
> > the last command until the next one arrives. A timeout would have the Uno
> > judge whether the brain is alive, and that is the brain's job.
> >
> > **Decided** (user, 2026-11-04): "If the Uno detects a End Strike or that the
> > cart has entered the safe zone. It needs to send the Orin a message and stop
> > the run." This replaces two decisions of 2026-11-03: "Run means RUN and do
> > not stop until stop is requested", and the gap being "a Uno guardrail, not a
> > run-ending event" (`real-loop-dia.md` lines 168–174).
> >
> > **Decided** (user, 2026-11-04): "Arduino goes into FAULT STATE on an end
> > strike of safe zone intrusion. Replied to ANY Orin Wifi Command with a FAULT
> > STATE response. Orin needs to acknowlege and stop the run." So the Orin, not
> > the Uno, ends the run.
> >
> > How the fault state works:
> >
> > - **The safe zone** is the `runtime_end_margin` pulses between each end's
> >   stop point and its cached trip position. It is the only zone the sources
> >   define near an end (`real-loop-dia.md` lines 395–396 and 403–407). The soft
> >   limit brings the cart to rest at the stop point and never past it, so a
> >   pulse count past a stop point is an intrusion: the count or the cache is
> >   wrong.
> > - **On an end strike or an intrusion the cart stops in one jump.** That is
> >   the rule for a trip, whose causes are the same, lost pulses or a wrong
> >   cache (`real-loop-dia.md` lines 421–423). Either way the cart is at rest,
> >   so the board can send.
> > - **In the fault state no command is applied.** Every datagram from the Orin
> >   gets the FAULT STATE reply instead, so a command can't move the cart.
> > - **The Orin's acknowledgement:** on a FAULT STATE reply, the brain stops
> >   sending commands, goes idle, closes the run, and `/run` shows the fault.
> > - **The fault clears at a reset of the board.** Every command is answered
> >   FAULT STATE, the seeks included, so nothing over WiFi clears it. A reset
> >   also forgets the rail, and the seeks start the next session, as every
> >   session starts (`real-loop-dia.md` lines 226–229).
> >
> > Follows from what is built:
> >
> > - **The message is a UDP datagram.** The bridge's push mode covers UDP only,
> >   so a WebSocket, which runs over TCP, is out (firmware README lines 158–166).
> > - **The board can send only with the cart at rest.** Sending costs it several
> >   exchanges with its WiFi chip, awaited in `poll_us` steps, during which it
> >   misses the 2 ms ticks (issue 6). In the fault state the cart is at rest,
> >   so its message and its replies cost no ticks.
> > - **A datagram can be lost,** up to 4.2% of them (above), so one message may
> >   not arrive.
> >
> > The steps:
> >
> > 1. **The firmware gets a state machine with four states** (user,
> >    2026-11-04):
> >    - **UNCALIBRATED:** the state at every start, since a reset forgets the
> >      rail. The seeks, the move to the centre and the text messages work as
> >      today. No command is applied, because without cached trip positions
> >      there is no soft limit. Each command gets the UNCALIBRATED reply.
> >    - **CALIBRATED:** entered when a seek pair has cached both trip positions,
> >      with the cart at rest. Commands are applied under the ramp and the soft
> >      limit, and a non-zero one starts MOVING. Text messages are answered as
> >      today.
> >    - **MOVING:** the Uno is sending any signal to the PUL pin. In MOVING it
> >      accepts no datagram except movement commands (user, 2026-11-04).
> >
> >      The agent's reading, not decided by the user:
> >      - MOVING is entered from UNCALIBRATED or CALIBRATED when pulses start:
> >        a seek, a move to the centre, or a command's non-zero rate.
> >      - The movement datagrams are `Stop` during a seek or a move to the
> >        centre, and the next command while commands drive the cart.
> >      - When the pulses stop it goes back to the state it came from. After a
> >        seek that trips the wrong switch or times out it goes to
> >        UNCALIBRATED, and after a completed seek pair to CALIBRATED.
> >    - **FAULT:** entered from MOVING on an end strike or a safe-zone
> >      intrusion. The cart stops in one jump. The board sends the FAULT STATE
> >      message once, to whoever sent the latest command (`wifi.rs`, `sender`,
> >      lines 292–307), then answers every datagram with it and applies none.
> >      Only a reset leaves it, back to UNCALIBRATED.
> >
> >    A lost message costs nothing, because the brain's next command gets the
> >    same reply. In UNCALIBRATED and FAULT the cart is at rest, so a reply costs
> >    no ticks.
> > 2. **The UNCALIBRATED and FAULT STATE replies use the board's one answer
> >    format,** the five keys (firmware README lines 32–60): `status` 409, "not
> >    possible as things stand" (line 57); `result` `"uncalibrated"` or
> >    `"fault"`; a `message` in words; `pin`, the switch that tripped, or null;
> >    and `pulses`, the count at the stop, or 0.
> > 3. **The brain, on either reply,** stops sending, goes idle and closes the
> >    run's log folder, and `/run` shows which: calibrate first, or the fault.
> > 4. `real-loop-dia.md` records all of this first, as the source of truth:
> >    - the board holds the last command, in place of the Open entry (lines
> >      432–434);
> >    - the fault state, in the sequence and in *Contracts*;
> >    - the two decisions of 2026-11-03 that these replace (lines 168–174).
> > 5. `real-arduino-plan.md` drops the entry (line 89) and gains the state
> >    machine. `real-orin-plan.md` gains the brain's handling of the reply.
> > 6. The sources of truth stop giving the Uno a Run. It has no concept of one
> >    (`real-loop-dia.md` lines 357–358), but these places give it one:
> >    - `real-loop-dia.md`: the section name *The command and the Arduino's
> >      Run* (line 301); "the firmware with Run" (line 125); "One firmware
> >      holds the seeks and Run" (line 364); "Run needs a finished calibration"
> >      (line 379); "A switch tripping during a run" (line 422); "The board's
> >      Run" in *What exists today* (line 573).
> >    - `real-arduino-plan.md`: its title, "the board's firmware for a balance
> >      run" (line 1); "needs a **Run**" (line 6); Done-when 5, "during a run"
> >      (line 30); A1, "the seeks and Run" (lines 40–41); A3's and A9's
> >      citations of *The command and the Arduino's Run* and "during a run"
> >      (lines 48 and 68).

---

> ## 6. Whether a command reaches the board during a fast move
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The command and the Arduino's Run*, lines 435–437:
> >   "Not checked on the board."
> > - `real-arduino-plan.md`, *Not decided by the diagram*, lines 90–91.
> > - `robot-plan.md`, *The Arduino's page*, Open, *STOP during Run*, lines
> >   88–99.
> > - `HANDOFF.md`, *Still open*, lines 237–238.
>
> **What the sources say:**
>
> - **The pulses never wait for the processor.** The HAL's step generator makes
>   and counts them in hardware (GPT320 and GPT321) and changes the rate at
>   each 2 ms tick (`__live__/arduino/rust/crates/calibration/src/pins.rs` lines
>   38–74; `HANDOFF.md` lines 105–113). The processor sleeps through a move and
>   wakes at each tick and at each byte from the WiFi chip (firmware README
>   lines 168–177).
> - **A datagram is already read during a move.** A seek reads `Stop` byte by
>   byte as each byte wakes it, without pausing the pulses (`stop_listener.rs`).
>   **Reported:** the WiFi link's receiver (`UartIrq`) works beside the step
>   generator on the HAL's validator (`HANDOFF.md` lines 170–172).
> - **Speeds run so far:** this firmware has run at most 10,000 pulses/s
>   (`calibration.toml` line 27). The motor has run 48,000 under the older
>   `calibrate` firmware (`jump-limit-results.md` lines 36–38). That firmware
>   made each pulse in software and couldn't hear STOP mid-move
>   (`bench-station-plan.md` lines 34–38). **Reported:** the HAL's STEP is proven
>   to 55,733 on its validator, not on the rig (`HANDOFF.md` lines 176–177).
> - **Follows, the load:** a command's frame from the WiFi chip,
>   `+UDPRX: <sock>,3|`, the 3 bytes and `\r\n`, is about 17 bytes, 1.5 ms at
>   115,200 baud, so it fits inside one 2 ms tick. At one command per camera
>   frame, 60 a second (`real-loop-dia.md` lines 80–83), the line is busy about
>   9% of the time.
> - **Follows, the rule the reader must keep:** today's `read_frame` waits in
>   `poll_us` steps (`wifi.rs` lines 395–427). That polling wait is allowed only
>   "never during a move" (`AGENTS.md`, *The firmware sleeps instead of
>   polling*). Answering costs the same kind of waits (`wifi.rs` lines
>   309–389), during which the board misses the tick the HAL needs polled at
>   least once per tick (`HANDOFF.md` lines 150–154).
> - **Follows, the timing against training:** in training a command acted on
>   its arrival step's tick (`synthetic-arduino-plan.md` lines 116–120). On the
>   board, `queue()` changes the rate at the next tick (`HANDOFF.md` lines
>   107–108). So a command takes effect up to one tick, 2 ms, later than in
>   training, which adds to D (issue 1).
> - **Not read, outside the repo:** the size of the HAL's receive ring, and how
>   the bridge fork pushes datagrams that arrive close together.
>
> > ### RESOLUTION PLAN
> >
> > **Answered by design:** a command reaches the board during a fast move. Step
> > 4 proves it on the rig.
> >
> > 1. **The command reader is a byte-by-byte parser in the move's wake loop,**
> >    as `StopListener` is. It matches `+UDPRX: <sock>,3|`, then counts exactly
> >    3 payload bytes, then `\r\n`. A payload byte can be `|`, `\r` or `\n`, so
> >    the payload is counted, not scanned, as `read_frame` already does
> >    (`wifi.rs` lines 391–394). `read_frame` is not used while the cart moves.
> > 2. **No command is dropped** (user, 2026-11-04: "If the new datagram does not
> >    violate the ramp, implement it, if it does violate the rame, re scope the
> >    ramp"). Each new command becomes the ramp's target as it arrives. If the
> >    change fits within one tick's step, m × 2 ms = 2,421 pulses/s, the next
> >    tick reaches it. If not, the ramp re-aims at it and moves towards it by at
> >    most 2,421 pulses/s a tick. A command that arrives before the last one is
> >    reached re-aims the ramp again. One `queue()` per tick, at least 40 µs
> >    before the next one (`HANDOFF.md` lines 152–154).
> > 3. **In MOVING the board takes only movement commands** (issue 5's state
> >    machine). It answers nothing and writes no flash, so nothing keeps the
> >    processor from each byte, and the receive ring can't fill, whatever its
> >    size.
> > 4. **The rig test:** commands sent at one per camera frame, reversing at up
> >    to 44,600 pulses/s for minutes. The camera's log shows the cart reversing
> >    with the commands, and a seek afterwards shows whether the pulse count
> >    stayed true.
> > 5. `real-loop-dia.md` records the answer in place of the Open entry (lines
> >    435–437), and `real-arduino-plan.md` drops it (lines 90–91).

---

> ## 7. What the board does with a command after a trip, or before a calibration
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The command and the Arduino's Run*, lines 438–440.
> > - `real-arduino-plan.md`, *Not decided by the diagram*, lines 92–93.
> > - `HANDOFF.md`, *Still open*, lines 239–240.
>
> **What the sources say:**
>
> - **Decided:** a switch tripping while the cart moves under commands is a fault, meaning lost pulses
>   or a wrong cache, and stops the cart in one jump (`real-loop-dia.md` lines
>   421–423).
> - **Built:** today the board forgets what it knows of the rail whenever a seek
>   trips the wrong switch or times out (`rail.rs` lines 55–64), because "something
>   is wrong". It keeps the span after a `Stop`, which loses no pulses (lines
>   65–70).
> - **Decided:** the soft limit needs both cached trip positions
>   (`real-loop-dia.md` lines 383–401). Before a calibration the board has none.
>   It forgets them at every reset (lines 226–229).
> - **Built:** today's `Rail` keeps only the span and the switch the cart rests on
>   (`rail.rs` lines 17–23), not the trip positions on one count.
>   `real-arduino-plan.md` step 1 is that work. **Reported:** each tick's
>   report from the HAL includes a `position` (`HANDOFF.md` line 152).
> - **Reported:** a pulse that started before a direction flip counts in the old
>   direction (`HANDOFF.md` lines 168–169). In one test run the HAL's test
>   instrument counted one edge more than the board, and the cause is not known
>   (lines 126–128). Balancing reverses the cart many times, unlike a seek, so
>   an error like that would add up until the next seeks. The soft limit trusts
>   the count.
>
> > ### RESOLUTION PLAN
> >
> > **Resolved by issue 5's state machine** (user, 2026-11-04):
> >
> > - **Before a calibration** the board is UNCALIBRATED: no command is applied,
> >   and each gets the UNCALIBRATED reply.
> > - **After a trip** the board is in FAULT: no command is applied, every
> >   datagram gets the FAULT STATE reply, and only a reset leaves it.
> >
> > **Decided** (user, 2026-11-04): "The Orin's /run page does not enable the Run
> > button unless the Arduino is in CALIBRATED state."
> >
> > 1. **The board answers a new text message, `state`,** with its state in the
> >    five-key answer format: `status` 200, `result` `"uncalibrated"`,
> >    `"calibrated"` or `"fault"`. At 5 bytes it can't be taken for a command.
> >    Like every answer, it is given only with the cart at rest, and the Run
> >    button matters only then.
> > 2. **The webserver forwards it** on `/run`'s status route (issue 2, step 3).
> >    The page enables Run only on `"calibrated"`. While Run is disabled, the
> >    page shows the reason beside it (user, 2026-11-04), one of:
> >    - "The board is not calibrated: run the calibration first."
> >    - "The board is in FAULT: reset the board, then calibrate." Only a reset
> >      leaves FAULT (issue 5).
> >    - "The board did not answer." No reply came within the wait: the board is
> >      off, or the message was lost.
> >    - "The brain did not answer." The webserver can't reach it.
> >    - "A run is going." Only STOP is enabled.
> > 3. `real-loop-dia.md` records it in place of the Open entry (lines 438–440),
> >    with `state` in *Contracts*.
> > 4. `real-arduino-plan.md` drops the entry (lines 92–93) and gains `state`.
> >    `real-orin-plan.md` gains the Run button's rule.

---

> ## 8. `runtime_end_margin`'s value
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The soft limit*, lines 408–409: "**Open:** its value,
> >   found on the rig through the settings page."
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 97.
> > - `HANDOFF.md`, *Still open*, line 244.
>
> **What it is:** the gap, in pulses, between each switch's cached trip position
> and the point where the soft limit stops the cart.
>
> **What the sources say:**
>
> - **Decided:** the soft limit caps the sent rate at v_cap = √(2 × m × d)
>   every tick, where d is the pulses left to the stop point
>   (`real-loop-dia.md` lines 383–401).
> - **Follows:** from the top speed the slow-down takes 44,600² ÷ (2 ×
>   1,210,937) = 821 pulses, 12.3 mm, before the stop point
>   (`real-loop-dia.md` lines 411–415).
> - **Follows, the cap's own rounding:** the cap is applied once per tick, and the
>   cart then runs for the whole tick. A rate of √(2 × m × d) held for 2 ms covers
>   more than d only when d < 2 × m × (0.002 s)² = 9.7 pulses. The worst
>   overshoot from this is about 2.4 pulses, 0.04 mm.
> - **Reported, the larger effect:** `queue(rate)` changes the rate at the next
>   tick, not at once (`HANDOFF.md` lines 107–108 and 160–165). So the cart runs
>   one more tick at its old rate before a lower cap takes hold. At top speed that
>   is up to 44,600 × 0.002 = 89 pulses, 1.3 mm, unless the rule works out d one
>   tick ahead.
> - **Configured:** the trip itself is confirmed `trip_us`, 1,000 µs, after the
>   press. That is 2 pulses at the seek's speed (`calibration.toml` lines 44–57),
>   so the cached trip position is about 2 pulses past the press.
> - **Follows:** a margin of 0 stops the cart exactly at the trip position,
>   where the switch is pressed, so the margin must be larger than 0.
> - **Measured, the count across many moves, with the older `calibrate`
>   firmware:** ten slow visits to switch D3 read the count 2, 5, 6, 8, 10, 13,
>   14, 16, 18, 20. The count climbed about 2 pulses per visit, and the cause is
>   not established (`jump-limit-results.md` lines 133–150). Passing trials lost
>   0 to 6 pulses each, apart from the first, which lost 11 (lines 107–111).
>   Reversals lost 3 to 5, and the cause of that is not established either
>   (lines 126–129).
> - **Measured, with this firmware:** the span, switch to switch, came out at
>   59,822 pulses "repeatably to one pulse" (`calibrate-w-homography.md` lines
>   133–134). That covers slow seeks only, not fast reversals.
> - **Follows:** balancing reverses the cart many times. If anything like the
>   older firmware's 2 to 5 pulses per move came back, hundreds of reversals
>   would shift the count by hundreds or thousands of pulses. Every 67 pulses
>   is 1 mm. The soft limit trusts the count, so the margin must cover the
>   drift until the next seeks.
> - **Measured, the conditions behind m:** the jump limit was measured with
>   jumps every 16 ms (`jump-limit-results.md` lines 36–37 and 166–168). The
>   board ramps every 2 ms, in jumps of 2,421 pulses/s instead of 19,374. A
>   smaller jump is easier on the motor (that file, lines 166–167: "A longer gap
>   is a bigger jump at the same j"), but braking at m every 2 ms has not been
>   run.
> - **Not known:** how far the switch's lever travels between releasing and
>   pressing again. `control-design.md` names the switches only as "lever
>   microswitches" (line 30), with no part number, so there is no datasheet to
>   read. The switch trips about 5 mm from the base
>   (`calibration_stop_gap`, `__synthetic__/trainer/python/settings.toml` lines
>   75–78), so the margin also has an upper end. A stop point more than about
>   333 pulses (5 mm) short of the trip leaves the policy that much less rail.
>
> > ### RESOLUTION PLAN
> >
> > **Resolved** (user): from top speed the cart stops in 821 pulses, 12.3 mm,
> > at the jump limit measured in the skip test (`real-loop-dia.md` lines
> > 411–415). The soft limit builds that in. `runtime_end_margin` is a setting:
> > it is set from the Arduino Settings page and kept in the Uno's data flash
> > (decided 2026-11-03, `real-loop-dia.md` lines 403–407). So its number is
> > chosen on the rig, and no design waits on it. The findings above are what
> > to keep in mind when choosing it.
> >
> > **Decided** (user, 2026-11-04): the default is 0.015 m, a constant in the
> > Uno's source code, used only when no setting is found in its data flash
> > (issue 11). The setting is in
> > pulses (`real-loop-dia.md`, *Contracts*, line 554), so that is 0.015 ÷
> > 0.000015 = 1,000 pulses at 15 µm a pulse. With it, the slow-down from top
> > speed starts 1,000 + 821 = 1,821 pulses, 27.3 mm, short of each trip
> > position.
> >
> > 1. `real-loop-dia.md` replaces its Open entry (lines 408–409) with this.
> > 2. `real-arduino-plan.md` drops the entry (line 97).

---

> ## 9. The settings messages over WiFi, and their answers
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The Uno's settings*, lines 481–482.
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 94; it blocks
> >   step 7, lines 139–142.
> > - `real-orin-plan.md`, *Not decided by the diagram*, line 106; it blocks
> >   step 9, line 145.
> > - `HANDOFF.md`, *Still open*, lines 241–242.
>
> **What the sources say:**
>
> - **Decided:** the board tells a command from a text message by length alone,
>   and no text message is 3 bytes (`real-loop-dia.md` lines 315–319). The text
>   messages today: `pins` and `Stop` 4 bytes, `MoveToCentre` 12, the two seeks
>   25 (`constants.rs` lines 6–24). A settings message must not be 3 bytes.
> - **A settings message can be JSON.** Only a command must be 3 bytes. Every
>   other datagram is told apart by its content. Two limits: the board keeps
>   only a datagram's first 32 bytes (`constants.rs` lines 30–32,
>   `REQUEST_MAX`), and `{"runtime_end_margin":1000}` is 27 bytes. The firmware
>   writes JSON but has no parser (its `Cargo.toml` lists none), so reading
>   JSON means a hand-written parser or a new `no_std` crate.
> - **Built:** a move's answer is JSON with five keys, `status`, `result`,
>   `message`, `pin` and `pulses`, and HTTP's status meanings (firmware README
>   lines 32–60). The webserver returns the board's `status` as the HTTP status
>   (`__live__/orin/rust/README.md` lines 87–93).
> - **Built:** the board answers whoever sent the latest datagram, so the
>   webserver keeps one exchange at a time (`__live__/orin/rust/README.md` line
>   99).
> - **Reported:** a data-flash write blocks the board until it ends, 1,024
>   bytes in 44 ms and a block erase in 6 ms (`real-loop-dia.md` lines 464–468).
> - **Follows** (`real-loop-dia.md` lines 472–474): a setting is changed only when
>   no run is going.
> - **Follows, with item 6:** the board can answer only while the cart is at rest,
>   because answering stops it servicing the tick. The board has no run state
>   (`real-loop-dia.md` lines 357–362). So the rule it can check for itself is
>   "the sent rate is 0".
>
> **Not known:** the messages' names and their format.
>
> > ### RESOLUTION PLAN
> >
> > **The messages: one text format, for both of issue 10's helpers.** One
> > settings function and one set of messages, read from WiFi or from USB.
> > Plain text needs no JSON parser in the firmware, and every message is longer
> > than 3 bytes, so none can be taken for a command.
> >
> > 1. **Pull:** `SETTINGS`, 8 bytes.
> > 2. **Change:** `SET runtime_end_margin <pulses>`, for example
> >    `SET runtime_end_margin 1000`, 27 bytes. That is within the 32 the board
> >    keeps (`REQUEST_MAX`) for values of up to 5 digits. A value that isn't a
> >    whole number from 1 to 99,999 is refused. The setting is written to the
> >    data flash, then read back.
> > 3. **The answer over WiFi** uses the board's five-key format (firmware README
> >    lines 32–60): `status` 200; `result` `"stored"` or `"default"`, saying
> >    which value is in use (issue 11, step 3); `pulses`, the value; `pin` null;
> >    and a `message` in words. A refused change answers 400, `"bad_value"`.
> > 4. **The answer over USB** is the manager's form (`arduino-manager`'s README
> >    lines 41–63): `OK SETTINGS <stored|default> <pulses>` and `OK SET
> >    <pulses>`.
> > 5. `real-loop-dia.md` records them in place of the Open entry (lines
> >    481–482) and in *Contracts*. `real-arduino-plan.md` drops its entry (line
> >    94), and `real-orin-plan.md` drops its entry (line 106).
> >
> > **Decided** (user, 2026-11-04):
> >
> > - **While the Orin is in Run mode, it accepts no change to the end zone.**
> > - **While the Uno is in MOVING, it accepts no datagram except movement
> >   commands** (issue 5's state machine). A change to the end zone sent then
> >   is dropped unanswered. The webserver's wait times out, and nothing
> >   changes.

---

> ## 10. How `arduino-manager` pulls and uploads the settings
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The Uno's settings*, lines 483–485.
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 95.
> > - `real-pi-plan.md`, *Not decided by the diagram*, lines 46–49; it blocks
> >   steps 1 and 2, lines 54–62.
> > - `__live__/rpi/rust/crates/arduino-manager/README.md`, *What it still
> >   needs*, lines 24–27.
> > - `HANDOFF.md`, *Still open*, lines 241–243.
>
> **What the sources say:**
>
> - **Built:** the manager speaks text lines at 115,200 baud over USB serial
>   (`HOME`, `MOVE`, `CENTRE`, `ENABLE`, `DISABLE`, `STOP`, `STATUS`). The board
>   is expected to reply `OK …` and to send `READY` at each start
>   (`__live__/rpi/rust/crates/arduino-manager/README.md` lines 41–63;
>   `src/protocol.rs`). Neither `calibrate` nor `calibration` speaks this
>   protocol (README lines 24–27).
> - **Built:** the `calibration` firmware's USB serial line, SCI9 at
>   `monitor_baud`, only writes. It prints start-up messages and failures, and
>   the firmware never reads from it (`main.rs` lines 57–69 and 128–160).
> - **Not known:** whether the HAL can receive on SCI9 the way `UartIrq` receives
>   on SCI1, with an interrupt that wakes the processor. That is the HAL's
>   answer.
> - **Configured:** opening the port at any rate but 1,200 or 2,400 baud does
>   not reset the board (`arduino-manager.toml` lines 30–33). So the manager can
>   talk to a running board without restarting it.
> - **Built:** after `POST /flash` the manager waits `port_wait_ms`, 15 s, for
>   the port to come back (`arduino-manager.toml` lines 53–55). "Upload firmware
>   and settings" would send the settings after that. The board has no `READY`
>   line to say it has started.
> - **Decided:** USB is in only while the rig is idle, because with the cable
>   in, both switch inputs have read open (`real-loop-dia.md` lines 476–477;
>   manager README lines 29–32).
> - **Follows:** the settings arrive by two channels, WiFi (item 9) and USB.
>   Both must mean the same and be answered the same, so the firmware either
>   has two readers or one reader fed by both.
>
> > ### RESOLUTION PLAN
> >
> > **Decided** (user, 2026-11-04): one function writes the setting, with two
> > helpers that feed it: one for WiFi from the Orin, one for USB from the Pi.
> >
> > 1. **The settings function** reads and writes `runtime_end_margin` in the
> >    data flash (issue 11), and refuses a change in MOVING (issue 9).
> > 2. **The WiFi helper** turns issue 9's settings datagrams into calls to that
> >    function, and its answer into a datagram.
> > 3. **The USB helper** does the same for the serial line, in the manager's
> >    protocol: one text line per command, answered `OK <command> <values>`
> >    (`__live__/rpi/rust/crates/arduino-manager/README.md` lines 41–63). The
> >    agent's names: `SETTINGS` to pull, and `SET runtime_end_margin <pulses>`
> >    to change.
> > 4. **The USB helper reads the serial line with the HAL's `UartIrq` on
> >    SCI9.** Today the firmware only writes SCI9, with the polled `Uart`
> >    (`main.rs` lines 57–69).
> >    - `UartIrq` is the HAL's interrupt-driven UART, with ring buffers, for any
> >      SCI channel (`D:\git\unor4-hal\README.md`, *UART*, lines 933–966).
> >    - The USB line reaches SCI9 through the ESP32-S3 bridge
> >      (`calibrate/calibrate.toml` lines 108–109; `bench-station-plan.md`
> >      lines 27–28).
> >    - So each byte from the Pi wakes the processor, as a byte from the WiFi
> >      chip does, with no polling.
> >    - Not the HAL's USB CDC-ACM: it takes the connector from the bridge, and
> >      uploads and the serial monitor disconnect (that README, lines
> >      1027–1030).
> > 5. **The manager's three options:**
> >    - pull sends `SETTINGS`;
> >    - firmware and settings: `POST /flash`, then, once the board's start-up
> >      lines appear in the serial log (`main.rs` lines 129–147), `SET` and a
> >      pull to confirm;
> >    - firmware only: today's `POST /flash`.
> > 6. `real-loop-dia.md` records it in place of the Open entry (lines 483–485).
> >    `real-pi-plan.md` drops its entry (lines 46–49), which unblocks its steps 1
> >    and 2, and `real-arduino-plan.md` drops its entry (line 95).

---

> ## 11. What the board uses when the data flash holds no setting
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The Uno's settings*, lines 486–487.
> > - `real-arduino-plan.md`, *Not decided by the diagram*, line 96; it blocks
> >   part of step 6, lines 133–138.
> > - `real-pi-plan.md`, *Not decided by the diagram*, line 50.
> > - `HANDOFF.md`, *Still open*, lines 242–243.
>
> **What the sources say:**
>
> - **Reported** (`real-loop-dia.md` lines 453–470): `blank_check()` says whether
>   bytes are erased. After a power loss or reset during a write or an erase,
>   that area must be erased again before it is trusted.
> - **Follows:** after a reset, the firmware can't tell that the reset
>   interrupted a write. A half-written setting is not blank, so
>   `blank_check()` passes it. Telling a whole setting from a half-written one
>   needs something stored with it, such as a check value written last.
> - **Decided:** `runtime_end_margin` is the only setting kept in the data flash
>   (`real-loop-dia.md` lines 403–407 and 444–446).
>
> > ### RESOLUTION PLAN
> >
> > **Resolved by issue 8's default** (user, 2026-11-04): when the data flash
> > holds no valid setting, the board uses the default, 0.015 m, 1,000 pulses.
> >
> > 1. **The default is a constant in the Uno's source code** (user,
> >    2026-11-04): 1,000 pulses, 0.015 m, a named constant in the firmware's
> >    `constants.rs` that cites this decision. It is used only when no valid
> >    setting is found in the data flash.
> > 2. At boot the board reads the setting's block. A value stored with a check
> >    value that holds is used. A blank block, or one whose check value fails
> >    because a reset cut a write short, gives the default.
> > 3. The Arduino Settings page shows which one the board is using: the stored
> >    value or the default.
> > 4. `real-loop-dia.md` records it in place of the Open entry (lines 486–487),
> >    with the default in *Contracts*.
> > 5. `real-arduino-plan.md` drops the entry (line 96), which unblocks its
> >    step 6. `real-pi-plan.md` drops its entry (line 50).

---

> ## 12. The log's file format, and the timestamp's form in the folder name
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The log*, line 502.
> > - `real-orin-plan.md`, *Not decided by the diagram*, lines 107–108; it blocks
> >   step 5, line 130.
> > - `HANDOFF.md`, *Still open*, lines 247–248.
>
> **What the sources say:**
>
> - **Decided** (user, 2026-11-03, `real-loop-dia.md` lines 491–497): one folder
>   per run in `runs` on the Orin, named by the timestamp of the Run press. Per
>   frame it holds the raw boxes from the detector, the coordinates after the
>   homography and the array sent to the policy.
> - **The repo's precedents:**
>   - The synthetic Orin logs one JSON object per line, one line per frame
>     (`observations.jsonl`, `synthetic-orin-plan.md` lines 179–187).
>   - The trainer names its run folders `YYYYMMDD_HHMMSS`, for example
>     `runs/20261101_163458` (`__live__/orin/python/README.md` lines 255 and
>     265).
>   - The dataset runs use `run_Set-NN_YYYYMMDD_HHMMSS` (`AGENTS.md`,
>     *Artifact naming*).
> - **From the plan:** the log is written a line at a time as each frame
>   happens, so a crash keeps everything before it (`real-orin-plan.md` step 5).
>   A file of one line per frame does that.
> - **Follows, a gap between the plan and the decision:** `real-orin-plan.md`
>   Done-when 2 rebuilds the 36 inputs "from the log's coordinates, its capture
>   times and the past actions". The decided contents don't name the capture
>   time or the command sent. The ages and the past actions are inside the 36
>   inputs. The last frame's command and each frame's own stamp are not.
> - **Built:** the Orin's Rust workspace already has `serde_json`
>   (`__live__/orin/rust/Cargo.toml` line 29), and the brain is Rust (issue 3).
>
> > ### RESOLUTION PLAN
> >
> > **The answer, from the precedents and the plans:**
> >
> > 1. **The folder:** `runs/YYYYMMDD_HHMMSS/`, from the Run press, in the
> >    trainer's form (`runs/20261101_163458`).
> > 2. **The file:** `frames.jsonl` in that folder, in JSON Lines: one JSON
> >    object per frame, written and flushed as each frame happens, so a crash
> >    keeps every line before it. The synthetic Orin's `observations.jsonl` is
> >    the precedent.
> > 3. **Each line holds the decided three,** as the brain has them:
> >    - `boxes`: per class, in class order, the detector's centre and size,
> >      normalised, and its confidence, or (−1, −1) for a marker not found;
> >    - `points`: Pivot's and End's (x, z) in metres, or (−1, −1);
> >    - `inputs`: the 36 numbers sent to the policy.
> > 4. **And three more fields that `real-orin-plan.md`'s checks need.** This
> >    adds to the list decided on 2026-11-03:
> >    - `stamp`, the frame's GStreamer timestamp. Done-when 2 rebuilds the
> >      inputs from the log's "capture times", and Done-when 4 checks them.
> >    - `a` and `c`, the policy's answer and the command sent. Done-when 3
> >      checks them against `policy.npz`.
> > 5. `real-loop-dia.md` records the format in place of the Open entry (line
> >    502) and in *Contracts* (lines 555–556). `real-orin-plan.md` drops the
> >    entry (lines 107–108), which unblocks its step 5.

---

> ## 13. The calibration's metres against a frame from the rig
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The calibration on the rig*, line 224: "This has not
> >   been checked against a frame from the rig."
> > - `calibrate-w-homography.md`, *The lens*, lines 222–224: the independent
> >   test "has not been done."
>
> - **The test** (`calibrate-w-homography.md` lines 220–224): check the matrix
>   "at a place it was not fitted to, such as a known pulse count from a
>   switch". It "has not been done."
> - **The fits recorded in the repo:**
>   - 2026-11-01, three stops, one number for the lens: worst point 0.166 px
>     (`calibrate-w-homography.md` lines 212–215). That doc warns the six points
>     look good "partly because they were fitted" (lines 217–219).
>   - The seven-pose fit with that one number failed, worst point 3.59 px
>     (`__live__/orin/python/README.md` lines 125–127).
>   - The lens was measured on 2026-11-02 with the Orin's `/lens` page. A
>     person holds the printed ChArUco board where a red box shows, flat in 25
>     positions and turned in 20, and OpenCV's `calibrateCamera` solves the
>     lens from the stills. The result was an average error of 0.26 px from 45
>     stills, in `lens.json` (`__live__/orin/python/README.md`, *Lens
>     calibration*, lines 138–205). A copy dated 2026-11-03 is
>     `__live__/orin/python/saved/lens-20261103-001338.json`.
>   - No fit since then is recorded in the repo. The fitted matrix,
>     `homography.json`, stays on the Orin and is not in git (that README, lines
>     129–136).
> - **Built:** the firmware moves only to a switch or to the centre (`wifi.rs`
>   lines 197–218). It can't stop the cart at a chosen count, so the doc's test
>   needs a new move first. `arduino-manager`'s protocol names a `MOVE <±pulses>
>   <rate>`, but no firmware speaks it (its README lines 24–27 and 47–51).
> - **Follows, the direction is consistent:** the fit gives the idler's stop the
>   negative `x` (`calibrate-w-homography.md` lines 122–127). The camera shows
>   the idler on the left, and the MJCF puts it at x = −0.485 (item 4). So `x`
>   grows to the right as the camera sees it, as the policy learned. Distances
>   away from the fitted points remain unchecked.
>
> > ### RESOLUTION PLAN
> >
> > 1. **The lens is done:** `/lens`, 2026-11-02, 0.26 px (above). It is measured
> >    again only if the lens or its focus changes ("remain fixed unless the
> >    camera optic is modified", `__live__/orin/python/README.md` lines
> >    145–147).
> > 2. **Fit the homography with that lens:** the seven poses on `/calibrate`,
> >    then Fit. It passes when `homography.json` says `"ok": true`, every point
> >    within `residual_px`, 1.0 px (`__live__/orin/python/settings.toml` line
> >    278).
> > 3. **Check it at places it wasn't fitted to.** The firmware gains a move by a
> >    set pulse count, `MoveBy <±pulses>`, built on the count stop `creep`
> >    already has (`seek.rs` lines 55–67, `Limit::Pulses`, the way
> >    `Rail::centre` uses it).
> >    - With the board CALIBRATED and at rest, move the cart to a quarter and
> >      to three quarters of the span.
> >    - At each, Pivot through the homography must match the `x` the count
> >      gives, `x_idler` + pulses × 15 µm, within 1 px of the frame, about 2.7
> >      mm (`calibrate-w-homography.md` line 25).
> > 4. `real-loop-dia.md` replaces line 224 with the result.

---

> ## 14. The data flash keeping its contents through a power-off
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The Uno's settings*, line 469: "Not yet run in the
> >   HAL: retention across power-off."
> > - `HANDOFF.md`, *Kept from the 2026-11-03 handoff*, line 262.
>
> - **Measured** (reported by the HAL): its checks on 2026-10-21 covered resets
>   and an upload, not a power-off (`real-loop-dia.md` lines 460–463).
> - **From Renesas's RA4M1 Group Datasheet** (R01DS0355EJ0110 Rev.1.10, Sep 29,
>   2023, section 2.15.2, Table 2.72, page 113; the copy read is
>   `D:\git\unor4-hal\Research\Renesas RA4M1 Datasheet - 1.10.md`, the one
>   `control-design.md` line 391 cites): data hold time 20 years after 10,000
>   erases of a block, and 5 years after 100,000, both at 85 °C. Endurance is
>   100,000 erases per block minimum, 1,000,000 typical. The data flash is
>   flash memory, which holds its contents with no power. Note 2 limits the
>   figures to "the flash memory programmer and the self-programming library
>   provided by Renesas Electronics". The firmware writes through `unor4-hal`
>   instead.
> - **From the same table's Note 1:** "programming the same address for several
>   times as one erasure is not enabled. Overwriting is prohibited." This
>   matches the HAL's `NotBlank` refusal (`real-loop-dia.md` line 461).
> - **From Table 2.73** (high-speed mode, 2.7–5.5 V): a 1 KB erase takes
>   6.42 ms typical and 237 ms at most, and programming one byte takes 42.1 µs
>   typical and 387 µs at most (FCLK 32 MHz). The HAL measured 6 ms and 44 ms
>   per 1,024 bytes, the typical figures. **Follows:** the worst case for one
>   1 KB erase and write is 237 ms + 1,024 × 387 µs ≈ 633 ms, not 50 ms. That
>   is the longest the board could be blocked by a settings write (items 9
>   and 11).
> - **What would settle it:** write a value, cut the power, read it back. The
>   datasheet covers the memory itself, and this test covers the firmware's
>   path through the HAL.
>
> > ### RESOLUTION PLAN
> >
> > **The test, with issue 9's messages:**
> >
> > 1. `SET runtime_end_margin 1234`, a value no default could give.
> > 2. Cut the board's power and restore it. The barrel jack is its supply
> >    (`control-design.md` line 343), and no USB cable may be in, since USB
> >    bypasses it (line 317).
> > 3. Once it has started, `SETTINGS` must answer `"stored"` with 1234. That
> >    also proves issue 11's boot rule on a value that survived.
> > 4. `SET` the intended value back.
> > 5. `real-loop-dia.md` replaces line 469 with the result.

---

> ## 15. A firmware-only upload leaving the settings in place
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The Uno's settings*, lines 469–470: "Still to test on
> >   the rig: `arduino-manager`'s firmware-only upload keeping the settings."
> > - `real-pi-plan.md`, P4, lines 34–37, and step 3, lines 63–65.
>
> - **Measured** (reported by the HAL, `real-loop-dia.md` line 462): an
>   `arduino-cli upload` left the data flash intact on the UNO R4 WiFi,
>   2026-10-21.
> - **Built:** the manager runs the same command, `arduino-cli upload -p <port>
>   --fqbn <fqbn> --input-file <bin>`
>   (`__live__/rpi/rust/crates/arduino-manager/src/flash.rs` lines 39–41), with
>   the FQBN `arduino:renesas_uno:unor4wifi` (`arduino-manager.toml` line 48).
> - **From Arduino's core for the board** (`ArduinoCore-renesas`, `boards.txt`
>   and `platform.txt`, main branch): `unor4wifi.upload.tool=bossac`, run as
>   `bossac --port=<port> -U -e -w <file>.bin -R`, with no offset.
> - **From BOSSA's source** (`src/bossac.cpp`): `-e` is "erase the entire flash
>   starting at the offset". But what is erased is up to the bootloader on the
>   board, which answers bossac's commands.
> - **From Arduino's UNO R4 bootloader** (`arduino-renesas-bootloader`,
>   `src/bossa.c`, "used by UNO R4 WIFI"):
>   - Its erase-all command `X` only echoes "X" and erases nothing.
>   - Erasing happens only as it writes (the `Y` command): each block-aligned
>     write first erases 2 blocks at `SKETCH_FLASH_OFFSET + offset`.
>   - `src/flash.h` sets `SKETCH_FLASH_OFFSET` to 16 KB, which matches
>     `memory.x`'s start at 0x4000 (the firmware's README, line 250).
>   - Nothing in the files read names the data flash.
> - **Follows:** an upload erases only the code flash it writes, from 16 KB up,
>   and leaves the data flash alone. That agrees with the HAL's measurement.
> - **Not read:** the body of `R_FLASH_LP_Erase`, in Renesas's FSP library,
>   which the bootloader calls.
> - **What would settle it on the rig:** pull, upload through the manager,
>   pull, as `real-pi-plan.md` step 3 says.
>
> > ### RESOLUTION PLAN
> >
> > **The test,** `real-pi-plan.md` step 3, with issue 10's USB messages:
> >
> > 1. `SETTINGS` over USB, and note the value and `stored`.
> > 2. Upload firmware only, today's `POST /flash`.
> > 3. Once the board's start-up lines appear in the manager's log, `SETTINGS`
> >    again. It must give the same value, still `stored`.
> > 4. `real-loop-dia.md` replaces lines 469–470 with the result.

---

> ## 16. STOP's order lets the brain restart the cart
>
> > ### ISSUE SOURCES
> >
> > - `real-loop-dia.md`, *The sequence*, lines 149–151. No Open list names it.
>
> In the sequence the webserver sends the board the command 0
> several times, and only then asks the brain to go idle. The brain keeps
> sending commands until it is idle, at about one per frame. A command sent after
> the zeros puts the cart back in motion, and nothing then sends 0 again. This
> follows from the diagram as drawn. It is not marked Open anywhere.
>
> > ### RESOLUTION PLAN
> >
> > **Reverse the order:** STOP first asks the brain for idle, so it sends no
> > more commands, and only then sends the board the command 0 several times.
> > The zeros go out whether or not the brain answered, so a brain that has
> > crashed still can't keep the cart moving. Nothing sends a command after the
> > zeros.
> >
> > 1. `real-loop-dia.md` swaps the two arrows in the sequence (lines 150–151)
> >    and the order in *The run and idle states* (lines 189–191).
> > 2. `real-orin-plan.md` swaps them in O8 (lines 80–82) and step 7 (lines
> >    139–140).
> > 3. Issue 3's diagram: done. Its two STOP arrows are swapped.

---

> ## 17. `HANDOFF.md`'s own open items, in a gitignored working file
>
> > ### ISSUE SOURCES
> >
> > - `HANDOFF.md`, *Still open*, lines 228–232 (the switches while the cart takes commands) and
> >   line 249 (the camera's trigger).
> > - `HANDOFF.md`, *Using it*, lines 155–157, and
> >   `__live__/arduino/rust/crates/calibration/src/pins.rs`, lines 76–95.
> > - `real-loop-dia.md`, *Timing*, lines 92–97 (the trigger, decided).
>
> - **How often the firmware reads the switches while the cart takes commands** (raised 2026-11-04;
>   not in the diagram). Its line 229 says a press can't wake the processor.
>   Its lines 155–157 and today's `pins.rs` (lines 76–95, `.with_wake()`) say the
>   HAL's wake-ups of 2026-11-04 make a press end the sleep. The two lines
>   disagree.
> - It still lists "whether the camera can be triggered". `real-loop-dia.md`
>   lines 92–97 record that as decided: not triggered (user, 2026-11-04).
>
> > ### RESOLUTION PLAN
> >
> > **Both items are answered:**
> >
> > - **The switches:** a press wakes the board at once. The switches are
> >   built with `.with_edge().with_wake()` (`pins.rs` lines 76–95), so a press
> >   ends the sleep, and the trip is confirmed `trip_us` later (`trip.rs`).
> >   At 44,600 pulses/s that is 44,600 × 0.001 ≈ 45 pulses, 0.67 mm, after the
> >   press. `HANDOFF.md` line 229 predates the HAL's wake-ups of 2026-11-04.
> > - **The camera:** not triggered (decided, `real-loop-dia.md` lines 92–97).
> >
> > 1. `HANDOFF.md` lines 228–232 and 249 are corrected, or dropped.

---

## Sources outside the repo

Read on 2026-11-04.

- **Leadshine, *DM542 Fully Digital Stepper Drive Manual*, V1.0, 2012** (item 4):
  copied into the repo as `research/dm542_2019-12-07.pdf` from
  <https://forum.linuxcnc.org/media/kunena/attachments/25011/dm542_2019-12-07.pdf>,
  the copy `control-design.md` cites.
- **Renesas, *RA4M1 Group Datasheet*, R01DS0355EJ0110 Rev.1.10, Sep 29, 2023**
  (issue 14): read in `D:\git\unor4-hal\Research\Renesas RA4M1 Datasheet - 1.10.md`.
  The original is <https://www.renesas.com/en/document/dst/ra4m1-group-datasheet>.
- **Arduino's core for the UNO R4, `ArduinoCore-renesas`, main branch** (issue 15):
  <https://raw.githubusercontent.com/arduino/ArduinoCore-renesas/main/boards.txt>
  and <https://raw.githubusercontent.com/arduino/ArduinoCore-renesas/main/platform.txt>.
- **BOSSA, the `bossac` upload tool** (issue 15):
  <https://raw.githubusercontent.com/shumatech/BOSSA/master/src/bossac.cpp> and
  <https://raw.githubusercontent.com/shumatech/BOSSA/master/src/Flasher.cpp>.
- **Arduino's UNO R4 bootloader, `arduino-renesas-bootloader`, main branch**
  (issue 15): <https://github.com/arduino/arduino-renesas-bootloader>, files
  `src/bossa.c`, `src/main.c` and `src/flash.h`.
- **GStreamer, `GstBaseSrc`** (item 1):
  <https://developer-old.gnome.org/gstreamer-libs/stable/GstBaseSrc.html>.
- **NVIDIA developer forum, not official documentation** (item 1):
  <https://forums.developer.nvidia.com/t/nvarguscamerasrc-timestamping/199563>
  and
  <https://forums.developer.nvidia.com/t/how-to-get-timestamp-using-nvarguscamerasrc/167865>.

- **`unor4-hal`'s README** (issue 10): `D:\git\unor4-hal\README.md`, *UART*
  (lines 933–966) and *USB CDC-ACM* (lines 1024–1030), read on the user's
  pointer.

Not read, because they are outside the repo:

- `unor4-hal`'s code: the receive buffer's size, and `queue()` beyond what
  `HANDOFF.md` reports.
- The user's bridge fork, `D:\git\unor4-wifi-interrupt-bridge`.
- Renesas's FSP library, which holds `R_FLASH_LP_Erase`.
