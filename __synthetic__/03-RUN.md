# 03-RUN — one balance run on the rig

A trained policy only proves itself on the real rig: the robot's computer, a
Jetson **Orin** Nano Super, watches the four markers through its camera, and
the policy turns what it sees into the cart's speed, swinging the pole up from
hanging and holding it upright, until someone presses STOP or something ends
the run. On 2026-11-09 the policy of training session `20261108_115227`,
6,000,000 steps, swung the pole up in 3.4 s and held it within 10° of upright
for the rest of a 30 s run (`__live__/orin/rust/crates/brain/README.md`). This file takes the rig from
switched on to one balance run and back: it checks that every part answers,
checks the camera's matrix, teaches the board where the rail's ends are,
starts the run, stops it, reads how it ended, puts the cart back in the
middle, and finds the run's log.

It is the last of three files, each starting where the one before ends
([01-TRAIN.md](01-TRAIN.md), then [02-DEPLOY.md](02-DEPLOY.md), then this
one). It starts where 02-DEPLOY.md ends: the Orin's **brain**, the program
that runs the balance loop, is running with the policy to try. It ends with
the run's log found; keeping that policy or going back to the one before is
02-DEPLOY.md's (*After the run*, at the end of this file). The
run itself is done from two pages the Orin's webserver serves, in a browser
on the PC. Each page explains its own buttons under *How to read this*; this
file says which to press, in what order, and what each should show. Where a
page's own words disagree with what its code does, this file says so.

A step whose title ends in *(not run yet)* has not yet been run as written
here.

> [!WARNING]
> The cart moves fast and with force. Stand at the **e-stop**, the rig's
> emergency stop switch, before pressing anything that moves it. The pages'
> STOP asks over WiFi, and a message can be lost, so it does not replace the
> e-stop.

> [!CAUTION]
> Not copy/paste on another rig: `192.168.0.215` is the author's Orin, its
> address on the network, and `tony` its user. Replace them with your Orin's
> in every link and command below.

## The parts

| Part | What it is | Needed for |
|---|---|---|
| The webserver | serves the pages at `http://192.168.0.215:8080/`, and passes each press on to the brain or the board | every step |
| The brain | runs the balance loop: each camera frame to the detector, the markers to the policy, the policy's speed to the board | Steps 1 and 4–6 |
| The board | the Arduino (an UNO R4 WiFi) that turns each speed into motor pulses, keeps the cart out of the ends, and stops it on a FAULT | every step |
| The calibration service | `~/live-orin-python/src/calibrate_service.py`: the camera and the detector for the calibrate page's pose measurements and live views | Step 2 only |

None of the three programs on the Orin starts by itself after the Orin
reboots. `__live__/orin/rust/DEPLOY.md`'s step 9 starts the webserver and the
brain, and [03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)'s Step 7
the calibration service. Step 1 shows whether the webserver and the brain are
running; Step 2a starts the calibration service when it is needed.

### The board's five states

The board's **state** is one of five words (the controller firmware's
`State::name`, `__live__/arduino/rust/crates/controller/src/state/mod.rs`):

- **`uncalibrated`**: it does not know where the rail's ends are. Every board
  starts so after it is switched on or reset, so each time the board is
  switched on, Step 3 comes before any run.
- **`calibrating`**: between the calibration's moves.
- **`calibrated`**: it knows both ends and the cart is at rest. Only now can
  a run start.
- **`moving`**: it is following the brain's speeds. Each time the cart comes
  to rest, it is `calibrated` again. While `moving` it throws away every
  message but a speed, so it does not answer "what state are you in?".
- **`fault`**: it stopped the cart by itself, in one jump, and stays so until
  it is reset (*After a FAULT*, at the end).

### What keeps the cart from the ends

The board counts every motor pulse, so it always knows where the cart is from
the idler end. One pulse is 0.015 mm of cart travel (the calibrate page's
*How to read this*). Each end has a **stop point** 1,000 pulses, 15 mm, short
of where its switch tripped during the calibration (`RUNTIME_END_MARGIN` in
`__live__/arduino/rust/crates/controller/src/constants.rs`). While the brain's
speeds move the cart, the board's **soft limit** brakes it to rest at a stop
point and never past it, and the run goes on.

The board goes into FAULT (`State::next` in the same `state/mod.rs`) when:

- the count passes a stop point: the cart entered that end's protected zone;
- a limit switch reads pressed while the board is `calibrated` or `moving`,
  including a broken switch wire, which reads the same, or someone touching
  a switch while the cart is at rest;
- a speed arrives whose first byte, its sign, is not 0 or 1.

### What ends a run

A run has no time limit. It ends on one of four things
(`__live__/orin/rust/crates/brain/src/run/run_loop.rs`):

- **STOP.** The webserver asks the brain to stop sending, then sends the
  board the speed 0 several times; the cart slows to rest.
- **A FAULT.** The board stops the cart itself and answers the brain with its
  FAULT reply.
- **The board's "not calibrated" reply.**
- **A failure in the brain:** the camera gives no frame for 2 s
  (`frame_timeout_ms` in `__live__/orin/rust/settings.toml`), the detector or
  the policy fails, or the log cannot be written. **The brain then stops
  without sending a 0, and the board has no time limit of its own: the cart
  keeps the last speed it was given until the soft limit brings it to rest at
  a stop point.** Step 6 says what to do.

> ## Step 1 — check every part answers (not run yet)
>
>> ### 1a. Unplug the board's USB cable from the Pi
>>
>> The **Pi** is a Raspberry Pi beside the rig that runs `arduino-manager`,
>> the program that uploads firmware to the board over its USB cable
>> (`__live__/arduino/rust/crates/controller/README.md`, *How it reaches the
>> board*). The cable is only for that. With it plugged in, both limit
>> switches have been seen to read open (the same README), and an open switch
>> is what a pressed one reads: pressing it opens its contact
>> (`__documentation__/control-design.md`). That refuses every seek and faults
>> a calibrated board. Unplug it before anything moves.
>
>> ### 1b. Open the run page
>>
>> Open <http://192.168.0.215:8080/run/> in a browser on the PC. Its title is
>> **Run the rig**.
>>
>> The page reads the state only when it opens and after each press of its
>> buttons. Reload it to read the state again.
>>
>>> #### Expected Results:
>>> Under **State**:
>>>
>>> - **Brain**: `Idle`.
>>> - **Board**: the board's state, then its own words, such as
>>>   `uncalibrated: …` after the board was switched on, or
>>>   `calibrated: the rail is known and the cart is at rest`.
>>>
>>> Beside the greyed-out **Run**, the reason it cannot be pressed yet, such
>>> as `The board is not calibrated: run the calibration first.`
>>>
>>> > [!WARNING]
>>> > - The page does not load at all: the webserver is not running. Start it
>>> >   in the Orin's terminal as `__live__/orin/rust/DEPLOY.md`'s step 9
>>> >   does (its first command); [02-DEPLOY.md](02-DEPLOY.md)'s *The Orin's
>>> >   terminal* logs in to the Orin.
>>> > - **Brain** `Not known: the brain did not answer, so it may not be
>>> >   running: …`: start it as [02-DEPLOY.md](02-DEPLOY.md)'s Step 7 does.
>>> > - **Board** `The board did not answer: …. It is off, or the message was
>>> >   lost.`: the board is off, its answer was lost, or it is `moving` and
>>> >   not answering. Watch the cart: if it is moving, press **STOP**. If it
>>> >   is still, check the board is switched on, then reload the page.
>>> > - **Board** `A calibration move is under way: the board hears nothing
>>> >   until the cart stops.`: a move started on the calibrate page is still
>>> >   running. Reload once the cart has stopped.
>>> > - **Board** `fault: …`: see *After a FAULT*, at the end.
>
>> ### 1c. The same, from the PC's terminal (optional)
>>
>> The page's state comes from the webserver's `/run/status`, which PowerShell
>> can ask for directly. `curl.exe` is named in full because PowerShell's own
>> `curl` is another command.
>>
>> > [!IMPORTANT]
>> > Run in the PC's terminal, PowerShell, from any folder
>>
>> ```pwsh
>> curl.exe -s http://192.168.0.215:8080/run/status
>> ```
>>
>>> #### Expected Results:
>>> One line of JSON. In it, `"state"` is the brain's, `"idle"`, and
>>> `"result"` inside `"board"` is the board's state, such as
>>> `"calibrated"`.

> ## Step 2 — the camera's matrix, only when the camera has moved (not run yet)
>
> The detector gives each marker in pixels, and the policy needs metres in
> the pole's plane. The **matrix**, `~/live-orin-python/homography.json` on
> the Orin, turns one into the other. It belongs to one camera position on
> one rig, so it stays on the Orin, and it is measured again only when the
> camera has moved. The brain reads it at every Run, and refuses the Run if it
> is missing, or did not pass its fit and was not accepted (Step 4's warning
> has the words). If the camera has not moved and the last Run was not
> refused for the matrix, skip to Step 3.
>
>> ### 2a. Start the calibration service
>>
>> The pose measurements need it. Start it as
>> [03-REAL_ORIN.md](../__object_detection__/03-REAL_ORIN.md)'s Step 7 does,
>> in the Orin's terminal (its first command).
>
>> ### 2b. Measure the seven poses
>>
>> On <http://192.168.0.215:8080/calibrate/>, with the cart at each end and
>> at the centre in turn, measure the pole in the seven poses the page asks
>> for. The fit runs after the seventh. The calibrate page's *How to read
>> this*, **Measuring the poses**, walks through it. The fit needs a lens
>> calibration from <http://192.168.0.215:8080/lens/>, and refuses to run
>> without one.
>>
>>> #### Expected Results:
>>> The fit passes. If it does not, the window offers **Accept this fit**;
>>> the brain uses an accepted fit as it does one that passed
>>> (`__live__/orin/rust/crates/brain/src/geometry/homography.rs`).
>
>> ### 2c. Close every live picture
>>
>> Close the pose window, and any `/live` or `/lens` page, then wait 10 s.
>> A live picture keeps the camera until 10 s after the last page asked for a
>> frame (`idle_stop_s` in `__live__/orin/python/settings.toml`). Before a
>> Run, the brain asks the calibration service only whether a pose is being
>> measured, not whether a live picture is open
>> (`__live__/orin/python/src/calibrate_service.py`, `is_busy`), so a Run
>> pressed meanwhile is not refused and may fail to open the camera.
>
> The poses' moves take the cart to both ends, and each seek that arrives
> records its end. If they found both ends, Step 3 needs only 3c.

> ## Step 3 — teach the board the rail (not run yet)
>
> Only when **Board** is `uncalibrated` or `calibrating`. For `fault`, see
> *After a FAULT* first: in FAULT every move is refused.
>
> The stepper motor cannot report where the cart is, so the board finds out:
> it runs the cart to the limit switch at one end, then to the other, counting
> the motor's pulses between them, and then moves to the middle. Only a move
> to the middle that arrives makes it `calibrated`. Each seek runs at 2,000
> pulses a second, 30 mm a second, and gives up after 34 s; on 2026-11-01 a
> full rail took 30.5 s (the calibrate page's *How to read this*).
>
> Open <http://192.168.0.215:8080/calibrate/>, titled **Calibrate the rail**.
> Each move adds a row under **What happened**, with its **Status** (200 when
> the cart did what was asked), **Result** and **Pulses**.
>
> > [!NOTE]
> > With the board's current firmware, the line under STOP keeps saying
> > `No answer from the board at …`, and the drawing cannot show the cart at
> > an end, which it does only from a switch that reads pressed.
> > Between moves the page asks the board for its switch readings again and
> > again (`poll_ms`, 1,000 ms between readings, in
> > `__live__/orin/rust/settings.toml`), and this firmware does not answer
> > that request (`__live__/arduino/rust/crates/controller/src/wifi/message.rs`).
> > The buttons still work. The **Centre** box's text says the move starts
> > from the switch the cart rests on; the board in fact centres from
> > anywhere once both ends are known.
>
>> ### 3a. Find the idler end
>>
>> Under **Idler end**, press **Find the idler end**. The cart runs slowly
>> towards the idler pulley until switch D3 trips.
>>
>>> #### Expected Results:
>>> A new row with **Status** `200` and **Result** `tripped`.
>>>
>>> > [!WARNING]
>>> > Any other row, from the board's answers
>>> > (`__live__/arduino/rust/crates/controller/src/wifi/answer.rs`) and the
>>> > webserver's (`__live__/orin/rust/crates/webserver/src/calibrate/mod.rs`):
>>> >
>>> > - `409` `already_there`: the cart was already resting on that switch,
>>> >   and the board only trusts a switch it sees trip during a move. Do 3b,
>>> >   then 3a again, then 3c.
>>> > - `409` `tripped`: the other end's switch tripped. The board has
>>> >   forgotten the rail: start Step 3 again.
>>> > - `409` `refused`: both switches read pressed. Check 1a.
>>> > - `408` `timed_out`: no switch tripped within 34 s. The board has
>>> >   forgotten the rail.
>>> > - `409` `stopped`: STOP was pressed.
>>> > - `409` `fault`: the board is in FAULT.
>>> > - `busy`: a move, a run or a measurement is under way.
>>> > - `no_answer`: the answer was lost on the WiFi, so the cart may have
>>> >   moved anyway. Look at the cart and the switches.
>
>> ### 3b. Find the motor end
>>
>> Under **Motor end**, press **Find the motor end**. The cart runs the whole
>> rail to switch D2.
>>
>>> #### Expected Results:
>>> **Status** `200`, **Result** `tripped`, and **Pulses** the rail's span,
>>> 59,821 on 2026-11-01. Any other row means what it does in 3a.
>
>> ### 3c. Move to the centre
>>
>> Under **Centre**, leave **Speed** empty, so the move runs at the seek
>> speed, and press **Move to the centre**.
>>
>>> #### Expected Results:
>>> **Status** `200` and **Result** `centred`: the cart stopped at the
>>> centre, and the board is now `calibrated`.
>>>
>>> > [!WARNING]
>>> > - `409` `not_ready`: the rail is not known yet: do 3a and 3b first.
>>> > - `409` `tripped`: a switch tripped on the way. The board has forgotten
>>> >   the rail: start Step 3 again.
>>> > - `409` `stopped`: STOP was pressed. The rail is still known: press
>>> >   **Move to the centre** again.
>>> > - `400` `bad_speed`: **Speed** holds something other than a whole
>>> >   number from 1 to the board's highest centring speed. Empty it.
>
>> ### 3d. Check on the run page
>>
>> Reload <http://192.168.0.215:8080/run/>.
>>
>>> #### Expected Results:
>>> **Board** `calibrated: the rail is known and the cart is at rest`, and
>>> beside **Run**: `The board is calibrated and the brain is idle.`

> ## Step 4 — start the run (not run yet)
>
>> ### 4a. Get ready
>>
>> - The cart is at the centre: 3c, or Step 7 after an earlier run, put it
>>   there.
>> - The pole hangs still, straight down.
>> - No live picture has been open on any page for the last 10 s (2c).
>> - You stand at the e-stop.
>> - On the run page, **Run** can be pressed: beside it,
>>   `The board is calibrated and the brain is idle.`
>
>> ### 4b. Press Run
>>
>> Press **Run** on <http://192.168.0.215:8080/run/>. From then on the brain
>> takes the camera's next frame after each command it sends, and sends the
>> board the speed the policy picks for it: about 30 a second, 30.0 on
>> average when measured on the rig (`__documentation__/real-timings.md`,
>> gap 9), from a camera that takes 60 frames a second
>> (`framerate` in `__live__/orin/rust/settings.toml`). The policy swings the
>> pole up from hanging, so the cart starts moving at once.
>>
>>> #### Expected Results:
>>> Under the buttons, `Run started.` Under **State**: **Brain** `Running`;
>>> **Run folder** the run's log folder on the Orin,
>>> `/home/tony/orin-runs/<YYYYMMDD_HHMMSS>` (Step 8); **Board**
>>> `Not asked during a run: the brain alone talks to the board then.`;
>>> and beside **Run**, `A run is going.` **Frames this run** is how many
>>> frames the brain has turned into a command; reload the page to see it
>>> grow.
>>>
>>> > [!WARNING]
>>> > `Run refused: …` means the brain did not start, and nothing moves. The
>>> > words say why (`__live__/orin/rust/crates/brain/src/run/brain.rs`,
>>> > `start`; `geometry/homography.rs`; `log/run_log.rs`):
>>> >
>>> > - `a run is going`: press STOP first.
>>> > - `the calibration service has the camera: finish or cancel its job
>>> >   first`: a pose is being measured.
>>> > - `no matrix at …: calibrate on /calibrate first (…)`,
>>> >   `the matrix in … did not pass and was not accepted: …`,
>>> >   `… was fitted on … frames, and the brain's are 728 x 544`,
>>> >   `… could not be read: …`, or `the matrix in … is not 3 x 3`: Step 2.
>>> > - `making the run's folder …`: two Runs pressed in the same second.
>>> >   Press Run again.
>>> > - `the brain did not answer, so it may not be running: …`: Step 1b.
>>> >
>>> > `Run: the Orin's webserver did not answer.` means the press did not
>>> > reach the Orin: Step 1b.
>>>
>>> > [!WARNING]
>>> > `Run started.`, then after a reload **Brain** `Idle` with a **Last
>>> > message**: the run started and failed at once, on the camera, the
>>> > detector or the board's link; the message says which. A live picture
>>> > still holding the camera is one cause (2c).

> ## Step 5 — stop the run (not run yet)
>
> Press **STOP**, the wide red button at the top of the run page. It stays
> pressable whatever the state.
>
>> #### Expected Results:
>> Under the buttons:
>> `STOP: The board was sent the command 0. The brain is idle.`
>>
>> "Sent" means the Orin sent the zeros, not that the board got them: WiFi
>> messages can be lost. Watch the cart either way.
>>
>> > [!WARNING]
>> > - The cart does not come to rest: press the e-stop now.
>> > - `The command 0 could not be sent: …. Use the e-stop.`: the board was
>> >   not told to stop. Press the e-stop now.
>
> If the cart stopped by itself before you pressed STOP, the run has already
> ended: go to Step 6.

> ## Step 6 — read how the run ended (not run yet)
>
> Reload the run page: it does not change by itself when a run ends.
>
>> #### Expected Results:
>> **Brain** `Idle`, and **Last message** one of:
>>
>> - `stopped`: STOP ended it. **Board** `calibrated: …`.
>> - `the board is in FAULT, at <pulses> pulses: reset it, then calibrate`:
>>   the board stopped the cart itself; `<pulses>` is where, counted from the
>>   idler end. **Board** `fault: …`. See *After a FAULT*.
>> - `the board is not calibrated: calibrate first`: do Step 3.
>>
>> **Frames this run** is how many frames the run acted on.
>>
>> > [!WARNING]
>> > Any other **Last message** is a failure in the brain, and the message
>> > says which. **The cart may still be moving at its last speed**
>> > (*What ends a run*): press **STOP** now, and the e-stop if the cart does
>> > not come to rest.

> ## Step 7 — put the cart back in the middle (not run yet)
>
> Only when **Board** is `calibrated`: Re-centre follows Run's rule.
>
> Press **Re-centre** on the run page. The cart moves to the middle of the
> rail at the seek speed, about 15 s from an end; both buttons stay grey
> until it has stopped. STOP ends the move too.
>
>> #### Expected Results:
>> Under the buttons, `Re-centre done: the cart stopped at the centre.`, and
>> beside **Run** again `The board is calibrated and the brain is idle.` The
>> next run starts from Step 4.
>>
>> > [!WARNING]
>> > `Re-centre did not arrive (…)`: the word in brackets is the board's
>> > result, as in 3c.
>> >
>> > - `stopped`: the board is `calibrating`, so Run and Re-centre stay grey,
>> >   though the run page's own text says Run waits for a Re-centre. The
>> >   rail is still known: press **Move to the centre** on the calibrate
>> >   page (3c).
>> > - `tripped`: the board has forgotten the rail: do Step 3 again.

> ## Step 8 — find the run's log (not run yet)
>
> Every Run the brain accepts makes a folder on the Orin, `~/orin-runs/` then
> the date and time Run was pressed, in UTC, as `YYYYMMDD_HHMMSS`
> (`[brain] runs_dir` in `__live__/orin/rust/settings.toml`); a refused Run
> makes none. Its `frames.jsonl` has one line per frame: what the detector
> found, what the policy was given, and the command it gave
> (`__live__/orin/rust/crates/brain/README.md`, *The log*). **Run folder** on
> the run page names it in full.
>
> To list the runs from the PC:
>
> > [!IMPORTANT]
> > Run in the PC's terminal, PowerShell, from any folder. `ssh` asks for the Orin's
> > password unless the PC has a key set up.
>
> ```pwsh
> ssh tony@192.168.0.215 "ls -1 orin-runs"
> ```
>
>> #### Expected Results:
>> One folder per accepted Run, oldest first, the newest last.

## After the run

The policy the brain ran is the PC's `__live__/orin/python/saved/policy.npz`,
not yet committed. What comes next is [02-DEPLOY.md](02-DEPLOY.md)'s:

- it balanced as it should: keep it (02-DEPLOY.md, *Keeping it*);
- it did worse than the one before: go back to that one (02-DEPLOY.md,
  *Going back*), then come back to this file for the next run.

## After a FAULT

A FAULT stops the cart in one jump and ends the run, and the board answers
every message with its FAULT reply until it is reset. A reset makes it
`uncalibrated`, so Step 3 follows, then Step 4 for the next run. Re-centre
cannot be pressed while the board is in FAULT.

> [!CAUTION]
> Not yet written: how to reset the board on the rig. Nothing in this repo
> says which reset to use.
