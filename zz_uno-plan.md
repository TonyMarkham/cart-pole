# The Uno and `arduino-manager`: the implementation plan

| | |
|---|---|
| **Status** | Plan, 2026-11-05. No work started. |
| **Purpose** | Turn every resolution plan in `real-open-issues.md` that touches the rig's Arduino or the Pi's `arduino-manager` into ordered work that can be built and checked. |
| **Source of truth** | `real-loop-dia.md` for the design; `real-open-issues.md` for the resolutions and the user's decisions (each issue's *OWNER APPROVED OVERRIDES*). This plan's own design is the agent's details (section 5), each with its reason and its mark. |
| **Waiting on the user** | Two decisions, asked in chat: A, whether `AGENTS.md`'s polling exception covers the FAULT message; B, approving the agent's choices D1, D2, D4, D5, D8 and D9. Each answer is recorded here as a ruling. Phase 0 starts once both are answered. The rig tests are the user's to run. |

## 1. Why this plan exists

The trained policy can only balance the real pole if the Arduino turns each
command into motor pulses the way training did, keeps the cart out of the ends
of the rail, and keeps its one setting through resets. The issues in
`real-open-issues.md` settled how. This plan puts those settlements in the
order the repo requires: the design into `real-loop-dia.md` first, then into
the plans written from it, then into code, then onto the rig, and the rig's
results back into the diagram.

## 2. Scope

**In scope:**

- every step in issues 1–16 that touches the Uno's firmware
  (`__live__/arduino/rust/crates/calibration`) or the Pi's `arduino-manager`
  (`__live__/rpi/rust/crates/arduino-manager`);
- the groundwork steps of `real-arduino-plan.md` and `real-pi-plan.md` those
  steps are built on;
- the document edits those steps need, in `real-loop-dia.md`,
  `real-arduino-plan.md`, `real-pi-plan.md`, `real-open-issues.md`,
  `robot-plan.md` and `AGENTS.md`.

**Out of scope, the Orin's work:**

- issues 2, 3, 12 and 16;
- issue 1 steps 1 and 3; issue 5 step 3; issue 7 step 2;
- issue 9's first Decided item (no change to the end zone while the Orin is
  in Run mode);
- issue 11 step 3 (the Orin's Arduino Settings page);
- issue 13 steps 1 and 2 (the lens, and the fit on `/calibrate`), and the
  `/homography` page;
- the Orin halves of issue 5 step 5, issue 7 step 4 and issue 9 step 5.

Where a rig test here needs the Orin, its row in section 8 says so.

**Left out:** issue 17, whose only step edits a file the issue doc says to
disregard (`real-open-issues.md` lines 13–16). Nothing in this plan rests on
that file: every fact comes from the repo's own documents, the firmware or
the HAL.

## 3. Conventions

**Sources.** Every step names one:

- **issue N step M**: that issue's resolution plan;
- **groundwork**: a step of `real-arduino-plan.md` or `real-pi-plan.md` that
  the issues are built on;
- **the order rules**: a design lands in `real-loop-dia.md` first
  (`real-loop-dia.md` lines 14–16; `real-open-issues.md` lines 27–30), and a
  plan is written from the diagram before code is written from the plan
  (`AGENTS.md`, *Truth flows one way*);
- **ruling R*n***: a user ruling (section 4);
- **detail D*n***: an agent's detail (section 5);
- **the HAL**: `unor4-hal`'s code, read read-only (R1).

**Marks in `real-loop-dia.md`**, as its lines 23–29 define them:

- a user decision lands as **Decided**, with its date and record;
- a part of a resolution plan with no mark lands as **Decided**, citing R2
  (the user's decision to implement the resolution plans as written);
- a part an issue marks **Follows** lands as **Follows**;
- "Resolved by an existing decision" (issue 5) lands as **Decided**, citing
  that decision of 2026-11-02;
- an agent's detail that is a consequence of decided things (D3, D6, D7)
  lands as **Follows**, with its working; D3's working also cites the HAL's
  code;
- an agent's detail that is a choice (D1, D2, D4, D5, D8, D9) lands as
  **Decided** once the user approves it (decision B). Until then it is
  **Proposed** and is not built (`AGENTS.md`, *Separate requirement from
  recommendation*: "A recommendation never hardens into an instruction").

**What goes into the diagram.** The design: states, rules, messages and their
answers. Implementation details below that level, such as a toml value's name
or where host tests live, go into the plans only.

**Order.** Diagram, then plans, then code, then rig tests, then the rig's
results into the diagram. Two steps keep their issue's own order instead:

- issue 4's rig test comes first, because its records and its code both need
  the result (issue 4); its code (step 1) then runs after its records (steps
  2–3), by the order rules;
- issue 6's plan entry is dropped after its rig test (issue 6 step 5).

**Code rules for every step** (`AGENTS.md`):

- tunable values in a toml, never in code, except the default the user put in
  code (R10);
- the firmware crate keeps one type per file, named after the type, with
  `use crate::…` first, a blank line, then the external crates;
- `cargo clippy --all-targets --no-deps` clean, and no `unwrap()` outside
  tests;
- live and synthetic share no code: the ramp is copied from the synthetic
  Arduino, never shared;
- the firmware sleeps in `WFI` until an interrupt, and every rule is a fixed
  rule per tick or per message (*The Uno is dumb*).

## 4. The user's rulings

| | Ruling | Recorded in |
|---|---|---|
| R1 | `D:\git\unor4-hal` may be read, read-only (2026-11-05). | `AGENTS.md`, *Read nothing on the user's computer outside this repo* |
| R2 | The resolution plans are implemented as written; an unmarked part needs no separate approval (2026-11-05). | `real-open-issues.md` lines 32–34 |
| R3 | Only entering the protected end zone is a fault; nothing else enters FAULT (2026-11-05). | issue 5 |
| R4 | How the code keeps the cart from passing a stop point is the agent's (2026-11-05). | issue 8 |
| R5 | In FAULT, `state` gets the FAULT STATE reply, like every WiFi datagram (2026-11-05). | issue 5 |
| R6 | Five states: UNCALIBRATED, CALIBRATING, CALIBRATED, MOVING and FAULT (2026-11-04 and 2026-11-05). CALIBRATING is entered from UNCALIBRATED or CALIBRATED by any calibration button on the Orin's calibration page, and left only for CALIBRATED, after a successful move to the centre. No transition goes from CALIBRATING to UNCALIBRATED, and FAULT is never entered from it. | issue 5 step 1, its state diagram and its overrides |
| R7 | `/homography` is a validation page only, holding the `MoveBy` check; the matrix comes only from calibration, on `/calibrate`, which gains a link button. `MoveBy` takes place only in CALIBRATED, never enters CALIBRATING, and runs in MOVING (2026-11-05). | issue 13 and its overrides |
| R8 | DIR's level is a `calibration.toml` key, set from issue 4's rig test; the 2026-10-29 run-time toggle is no question (2026-11-05). | issue 4 |
| R9 | Host tests of the firmware's pure rules run on the PC, as checks only, in a small `no_std` library crate beside the firmware that the firmware itself uses (2026-11-05). | `AGENTS.md`, *Repository map*, `__live__/` |
| R10 | The default margin, 0.015 m, 1,000 pulses, is a constant in the Uno's source code (2026-11-04), against `AGENTS.md`'s toml rule for this one value. | issues 8 and 11; `AGENTS.md` once P0.6 is done |
| R11 | In MOVING, the board takes only movement commands, and a settings change sent then is dropped unanswered (2026-11-04). | issues 5 and 9 |
| R12 | No command is dropped in MOVING: each re-aims the ramp (2026-11-04). | issue 6 |
## 5. The agent's details

No issue gives these. Each is a fixed rule, built in Phases 1–2. Those marked
*choice* wait on decision B.

| | Detail | Why | Kind |
|---|---|---|---|
| D1 | **Answers to a `SET` that fails.** Over WiFi, a read-back that doesn't match answers 500, `result` `"write_failed"`, `pulses` the value read back, `pin` null. Over USB, a refused value answers `ERR SET bad_value`, and a read-back that doesn't match `ERR SET write_failed`; the manager's protocol gains the line `ERR <command> <reason>`. | Issue 9 steps 2–4 give none of these. 500 is the board's status for its own failure (the firmware README). | choice |
| D2 | **A command in CALIBRATING, at rest between moves,** is not applied and gets a 409 `"calibrating"` reply in the five-key format. During a calibration move it is dropped, as every datagram but `Stop` is (issue 5). How the brain handles the reply is the Orin's (issue 5 step 3). | Only CALIBRATED leads to MOVING (R6); a command in UNCALIBRATED gets its state's reply the same way (issue 5 step 2). | choice |
| D3 | **The soft limit's bounds.** The rule stays the decided cap, √(2 × m × d), applied while the cart heads for an end, with d the pulses left to that end's stop point; moving away from an end is never capped (`real-loop-dia.md` lines 389–400). Two bounds are added so the cart never passes a stop point. A rate queued now takes effect at the next tick (the HAL, `step_generator.rs` line 573), so d is worked out one tick ahead: d₁ = d − v × 2 ms, with v the rate running now. And one tick may not cover more than d₁, so the rate is at most d₁ ÷ 2 ms. Heading for an end, the rate queued is the smallest of the ramp's, √(2 × m × d₁) and d₁ ÷ 2 ms, and 0 once d₁ ≤ 0. | R3 and R4; issue 8's 2.4-pulse rounding, which the second bound removes. | Follows |
| D4 | **`MoveBy`.** One whose end would lie past a stop point is refused, 400, `result` `"bad_pulses"`, and the cart stays put. Sent in UNCALIBRATED, CALIBRATING or FAULT it gets that state's reply; sent in MOVING it is dropped unanswered (issue 6 step 3, R11). While it runs, a command ends it and takes over through the ramp from the `MoveBy`'s rate (R12); the command 0, which `/homography`'s STOP sends (issue 13), brings the cart to rest. | It must never enter the zone (R3), and the soft limit binds the commands only (`real-arduino-plan.md` A8); it takes place only in CALIBRATED (R7). | choice |
| D5 | **A 3-byte datagram whose byte 0 is neither 0 nor 1** is dropped, unless its size is 0, which is rest whatever the sign byte says (`real-loop-dia.md` line 323). | The contract defines the sign as 0 or 1 (`real-loop-dia.md` line 312). | choice |
| D6 | **The idle sleep watches both receive rings**, the WiFi chip's and the Pi's. | Issue 10 step 4 has a byte from the Pi wake the board; the sleep must then look at its ring. | Follows (plans only) |
| D7 | **The manager's wait** for the board's start-up lines after a flash is a value in `arduino-manager.toml`. | Issue 10 step 5 waits for those lines; tunable values go in a toml. | Follows (plans only) |
| D8 | **USB lines by state.** With the cart at rest, `SETTINGS` and `SET` over USB are answered in every state, FAULT included. While the cart moves, they are dropped unanswered. | The FAULT reply covers WiFi commands only (issue 5's quote). The board never waits on a line while the cart moves (`AGENTS.md`, *The firmware sleeps instead of polling*), and R11 drops a WiFi change then. | choice |
| D9 | **Text messages in CALIBRATING at rest** between moves (`pins`, `state`, `SETTINGS`, `SET`) are answered as in UNCALIBRATED and CALIBRATED. | The cart is at rest, so answering breaks no rule (`AGENTS.md`, *The firmware sleeps instead of polling*). | choice |

## 6. Conflicts, and how this plan resolves them

| Conflict | Resolution |
|---|---|
| Issue 7 step 1 answers `state` 200 with `"fault"`; issue 5 answers every WiFi datagram in FAULT with the FAULT STATE reply. | R5: in FAULT, `state` gets that reply. Issue 7's 200 holds outside FAULT. |
| Issue 5's 2026-11-04 statements name an end strike as a FAULT cause; its 2026-11-05 statement makes entering the zone the only one. | Issue 5 records the replacement (R3). |
| Issue 6's "no command is dropped" (R12); issue 5 drops every datagram but `Stop` during a calibration move. | Issue 6's override is scoped to MOVING; a calibration move is CALIBRATING (R6). |
| Issue 10 step 1 and `real-arduino-plan.md` step 7 say a change in MOVING is "refused"; issue 9's Decided item says "dropped unanswered". | R11: dropped unanswered. |
| Issue 4 step 1 compiles DIR's level in; `real-loop-dia.md` lines 427–431, under **Open**, and `robot-plan.md` lines 53–59 hold the run-time toggle that issue 4 records as Decided (2026-10-29). | R8. Issue 4 step 2 replaces the diagram's lines, and P0.5 marks `robot-plan.md`'s. |
| The decided cap can overshoot a stop point by 2.4 pulses through its rounding, and a rate queued now runs a tick later; R3 makes either a fault. | R4 and D3. The cap stays the rule; D3 adds two bounds. Issue 8's override, "nothing decided", is reworded in P0.5. |
| R6 applies no command in UNCALIBRATED, CALIBRATING or FAULT, and answers it there; `real-arduino-plan.md` step 5 applies every 3-byte datagram, and issue 1 reads the command contract (`real-loop-dia.md` line 553, "no id") as a command getting no answer. | Issue 5's override names the diagram's lines 357–360, A10 and Done-when 1; Phase 0 also brings step 5 and line 553 into line. |
| Issue 5 step 6's list of places that give the Uno a Run leaves out `real-arduino-plan.md` A2's citation (lines 43–44) and `real-loop-dia.md` lines 383–384 and 407–408. | Phase 0 includes them. |
| Issue 9 step 2 accepts 1 to 99,999 pulses. A margin of 1 or 2 puts the stop point at or past the switch's press, about 2 pulses short of the trip position (issue 8), against "the switch is never reached" (`real-loop-dia.md` lines 398–399). A margin of 29,911 or more, half the 59,822 span (issue 8), puts each stop point at or past the centre, so the centre, where CALIBRATING ends, lies inside the protected end zone. | Issue 8: the value is chosen on the rig, with these findings in mind. The range stands as written (R2). |
| Issues 5 and 16 both edit `real-loop-dia.md` lines 189–191 and the sequence: issue 5 retires "STOP is the only way out of run" and adds the fault state; issue 16 swaps STOP's two arrows. | They don't contradict. P0.2 makes issue 5's edits; issue 16's is the Orin's and lands with its own work. |
| `AGENTS.md`, *Tunable values live in a toml*, keeps every changeable value out of code; R10 puts the default in code. | R10 stands; P0.6 records the exception in `AGENTS.md`, where rules about the repo belong (*Anything about this repo is recorded in this file*). |
| Issue 5 has the board send the FAULT message once, unprompted; `AGENTS.md`, *The firmware sleeps instead of polling*, allows the AT-command wait only "at the join and around an answer to the Orin". | Decision A. |

## 7. Coverage

Every in-scope step, and where this plan carries it out.

| Issue | Steps | Where |
|---|---|---|
| 1 | 2 (the toggle on pin D13), 4 (record) | P0.2, P0.3, P1.6, P3 |
| 4 | the rig test, 1, 2, 3 | P0.1, P0.5, P1.1 |
| 5 | 1, 2, 4, 5 (Uno), 6 | P0.2, P0.3, P1.3, P1.5, P1.6 |
| 6 | 1, 2, 3, 4, 5 | P0.2, P0.3, P1.6, P3 |
| 7 | 1, 3, 4 (Uno) | P0.2, P0.3, P1.3 |
| 8 | 1, 2, the default, R4 | P0.2, P0.3, P0.5, P0.6, P1.5, P1.7 |
| 9 | 1–4, 5 (Uno), the Decided item on MOVING | P0.2, P0.3, P1.8, P2.1 |
| 10 | 1–6 | P0.2–P0.4, P1.8, P2.1, P2.2 |
| 11 | 1, 2, 4, 5 | P0.2–P0.4, P0.6, P1.7 |
| 13 | 3 (`MoveBy`), 4 (record) | P0.2, P0.3, P1.9, P3 |
| 14 | 1–5 | P3 |
| 15 | 1–4 | P3 |

## 8. The work

### Phase 0: the rig test for DIR, then the documents

Phase 0 starts once decisions A and B (the header) are answered.

1. **Issue 4's rig test** (the user's), first (section 3, *Order*). Send
   `CalibrateIdlerTravelLimit`, which today's firmware runs with D5 high, and
   watch which way the cart goes. Then `real-loop-dia.md` replaces its Open
   entry, lines 427–431 (issue 4 step 2), and `real-arduino-plan.md` drops its
   entry, line 88 (issue 4 step 3).
2. **`real-loop-dia.md`:**
   - issue 5 step 4: the board holds the last command; the fault state in the
     sequence and in *Contracts*, with its one cause (R3) in place of lines
     421–423 and of line 143's "A switch tripping (a fault)"; the 2026-11-03
     decisions it replaces (lines 168–174); the Open entry at lines 432–434;
   - the five states and their diagram (R6, issue 5 step 1), `MoveBy`'s
     transition included (R7);
   - the passages issue 5's overrides name: lines 176–187, 189–191, 357–360
     and 416–420;
   - issue 5 step 6: the wording that gives the Uno a Run goes, in the places
     it lists and in lines 383–384 and 407–408;
   - *Contracts*, the command (lines 551–553): a command gets no answer in
     CALIBRATED and MOVING, and its state's reply in UNCALIBRATED, CALIBRATING
     and FAULT (issue 5 step 2, D2);
   - issue 6's design: the parser, and each command re-aiming the ramp in the
     ramp rule, lines 332–337 (issue 6's override);
   - issue 7 step 3, with `state` in *Contracts*, `"calibrating"` included
     (issue 7 step 1, Follows);
   - issue 8 step 1, with D3;
   - issue 9 step 5;
   - issue 10 step 6;
   - issue 11 step 4, with the default in *Contracts*;
   - lines 453–454: the HAL is read, read-only (R1), in place of "the HAL
     itself was not read";
   - by the order rules: the LED toggle on pin D13 (issue 1 step 2), `MoveBy`
     (issue 13 step 3), and D1, D2, D4, D5, D8 and D9, each with its mark
     (section 3); D3 lands with issue 8 step 1, and D6 and D7 go into the
     plans only (section 3, *What goes into the diagram*).
3. **`real-arduino-plan.md`:**
   - issue 5 step 5: drops its entry, line 89, and gains the state machine;
   - issue 5 step 6: no Run in the places it lists, nor in A2's citation
     (lines 43–44);
   - A10, Done-when 1 and step 5 brought into line with R6;
   - A9, step 4's "A trip stops the cart in one jump" and Done-when 5 brought
     into line with R3;
   - A4: each command re-aims the ramp (issue 6's override);
   - step 2: the HAL is read, read-only (R1), in place of "asked of the user,
     not read";
   - step 6: its "blocked on" note goes (issue 11 step 5);
   - step 7: its two "blocked on" notes go, since issues 9 step 5 and 10 step 6
     drop the entries it is blocked on (lines 94 and 95); its "refused"
     becomes "dropped unanswered" (R11);
   - issue 7 step 4: drops its entry, lines 92–93, and gains `state`;
   - drops the entries named in issues 8–11, lines 94–97; issue 6's, lines
     90–91, stays until its rig test;
   - by the order rules: issue 6's parser; the settings function and its WiFi
     and USB helpers (issue 10); the check value and the default constant
     (issue 11); the LED toggle (issue 1); `MoveBy` (issue 13); the host-test
     crate (R9); D1–D6, D8 and D9 (D7 goes to `real-pi-plan.md`).
4. **`real-pi-plan.md`:** drops its entries, lines 46–50 (issue 10 step 6,
   issue 11 step 5); steps 1 and 2 lose their "blocked on" notes (issue 10
   step 6); gains issue 10 step 5's three options, D1's `ERR` line and D7.
5. **`real-open-issues.md` and `robot-plan.md`:**
   - issue 8's override: "*Changes:* nothing decided; the cap, √(2 × m × d),
     stays" becomes "the cap, √(2 × m × d), stays; the code adds the bounds
     that keep the cart short of a stop point (R4, D3)";
   - `robot-plan.md` lines 53–59, the Left / Right toggle: a note that issue
     4 replaces it (R8), as issue 4's override names.
6. **`AGENTS.md`:**
   - *Tunable values live in a toml*: the exception for the
     `runtime_end_margin` default (R10), as issues 8 and 11's overrides name;
   - *The firmware sleeps instead of polling*: the FAULT message, as decision
     A answers.

**Done when:** the rig test is recorded, and every document above carries
every item listed for it, with its mark (section 3).

### Phase 1: the firmware (`__live__/arduino/rust/crates/calibration`)

0. **STEP from the GPT timer, with a pulse count** (groundwork, step 2). Built
   in part: `pins.rs` drives STEP through the HAL's step generator, and GPT321
   counts every pulse. The signed count the cached rail needs is the HAL's
   `position()`, "completed pulses forward minus completed pulses backward"
   (`step_generator.rs` lines 852–854).
1. **`calibration.toml`:** `jump_limit` (groundwork, step 3), and DIR's level
   for a positive command from P0.1, compiled in by `build.rs`, with
   `Direction::FORWARD` (`direction.rs` line 18) taking it (issue 4 step 1,
   R8).
2. **The cached rail:** both trip positions and the centre on `position()`,
   and `MoveToCentre` from anywhere (groundwork, step 1).
3. **The state machine** (issue 5 step 1, R6):
   - the five states, as issue 5's diagram draws them;
   - a seek or `MoveToCentre` enters CALIBRATING; a successful `MoveToCentre`
     leaves it for CALIBRATED;
   - during a calibration move only `Stop` is taken, and a switch's trip ends
     the move by its own rules, never as a fault (issue 5); text messages work
     as today in UNCALIBRATED and CALIBRATED, with the new ones added by issues
     7, 9 and 13, and in CALIBRATING at rest (D9);
   - the `uncalibrated` and `fault` replies, 409, in the five-key format, with
     `pin` the switch that tripped or null, and `pulses` the count at the stop
     or 0 (issue 5 step 2); in FAULT every WiFi datagram gets the FAULT STATE
     reply (R5);
   - the FAULT message once, to the latest command's sender, with the cart at
     rest (issue 5 step 1, decision A); only a reset leaves FAULT, to
     UNCALIBRATED;
   - the `state` message (issue 7 step 1), `"calibrating"` included.
4. **The ramp**, copied from the synthetic `Ramp`, ticked every 2 ms
   (groundwork, step 3).
5. **The soft limit**, after the ramp (groundwork, step 4), with D3's bounds.
   In MOVING, the count passing a stop point is the cart entering the
   protected end zone, checked every tick: the board goes to FAULT and stops
   the cart in one jump (issue 5, R3).
6. **The command path:**
   - at rest, a 3-byte datagram is a command (groundwork, step 5), applied in
     CALIBRATED only; in UNCALIBRATED and FAULT it gets that state's reply
     (issue 5), in CALIBRATING D2's; D5 for a byte 0 other than 0 or 1;
   - in MOVING, a byte-by-byte parser in the wake loop matches
     `+UDPRX: <sock>,3|`, counts exactly the 3 payload bytes, then `\r\n`, and
     drops everything else (issue 6 steps 1 and 3);
   - each command re-aims the ramp, and none is dropped (issue 6 step 2, R12);
     one `queue()` per tick, at least 40 µs before the next one (issue 6 step
     2), as the HAL requires: it refuses a second change in a tick, and any
     change within 40 µs of the tick (`step_generator.rs` lines 609–612);
   - pin D13, the board's LED, toggles once per command (issue 1 step 2).
7. **The setting in the data flash** (groundwork, step 6): stored with a check
   value written last; a blank or failed check gives the default (issue 11
   step 2), a 1,000-pulse constant in `constants.rs` citing the 2026-11-04
   decision (R10).
8. **The settings function** and its two helpers (issue 10 steps 1–4):
   - a change sent in MOVING is dropped unanswered (R11);
   - over WiFi: `SETTINGS` and `SET runtime_end_margin <pulses>`; a value
     outside 1 to 99,999 is refused, and a good one is written, then read back
     (issue 9 steps 1–2). The answer is the five-key format: `status` 200,
     `result` `"stored"` or `"default"`, `pulses` the value, `pin` null, and a
     `message`; a refused value is 400, `bad_value` (issue 9 step 3); a failed
     read-back is D1's;
   - over USB: `UartIrq` on SCI9, so each byte from the Pi wakes the processor
     (issue 10 step 4), answered `OK SETTINGS …` and `OK SET …` (issue 9
     step 4), with D1, D6 and D8.
9. **`MoveBy <±pulses>`** on `creep`'s count stop, the way `Rail::centre` uses
   it (issue 13 step 3); taken only in CALIBRATED and running in MOVING (R7),
   by D4.
10. **The README** (groundwork, step 8), written for a layman.

Host tests of the ramp, the soft limit and the command unpacking, in the
library crate (R9).

**Done when:** `cargo clippy --all-targets --no-deps` is clean from the repo
root; the host tests pass; the firmware builds with `cargo build --profile
firmware`, the firmware README's build command; and `real-loop-dia.md`'s
*What exists today* rows for the board's command path and the setting in the
data flash (lines 573–574) say what is built.

### Phase 2: `arduino-manager` (`__live__/rpi/rust`)

1. **`protocol.rs`:** `SETTINGS` and `SET`, their `OK` replies (issue 9 step
   4, issue 10 step 3), and D1's `ERR <command> <reason>` line.
2. **The three options** (issue 10 step 5; `real-pi-plan.md` steps 1–3):
   - pull: sends `SETTINGS`;
   - firmware and settings: `POST /flash`, then, once the board's start-up
     lines appear (D7), `SET` and a pull to confirm; refused while another
     action runs, as a flash is today (`real-pi-plan.md` step 2);
   - firmware only: today's `/flash`.
3. **The page** shows the pulled settings (`real-pi-plan.md` step 1).
4. **The README** (`real-pi-plan.md` step 4), written for a layman.

**Done when:** `cargo clippy --all-targets --no-deps` is clean from
`__live__/rpi/rust`, and `real-loop-dia.md`'s *What exists today* row for the
manager's three options (line 575) says what is built.

### Phase 3: the rig tests (the user's), and their results in the diagram

| Test | Needs | Its record afterwards |
|---|---|---|
| **Issue 1 step 2:** the LED in the camera's view; over many commands, the frame first showing a toggle less the frame whose command caused it; the mean less 8.3 ms is D. | the brain's log of each command's frame (the Orin) | `real-loop-dia.md` line 107 and lines 504–515 (issue 1 step 4) |
| **Issue 6 step 4:** commands at one per camera frame, reversing at up to 44,600 pulses/s for minutes; the camera's log shows the cart reversing with the commands, and a seek afterwards shows whether the count stayed true. | the Orin, to send one command per frame | `real-loop-dia.md` lines 435–437, and `real-arduino-plan.md` lines 90–91 dropped (issue 6 step 5) |
| **Issue 13:** the homography fitted on `/calibrate` to `"ok": true` (step 2); then, on `/homography`, with the board CALIBRATED and at rest, `MoveBy` to ¼ and ¾ of the span; at each, Pivot through the homography matches `x_idler` + pulses × 15 µm within 1 px, about 2.7 mm (step 3). | `/calibrate` and `/homography` (the Orin) | `real-loop-dia.md` line 224 (issue 13 step 4) |
| **Issue 14:** `SET runtime_end_margin 1234`; cut the power and restore it, with no USB cable in while it is cut (step 2); `SETTINGS` answers `"stored"`, 1234; `SET` the intended value back (steps 1–4). The messages go over USB or WiFi (issue 9). | `arduino-manager` (Phase 2), or the Orin | `real-loop-dia.md` line 469, the retention sentence (issue 14 step 5) |
| **Issue 15:** `SETTINGS` over USB; upload firmware only; once the board's start-up lines appear in the manager's log, `SETTINGS` again: the same value, still `stored` (steps 1–3). | `arduino-manager` (Phase 2) | `real-loop-dia.md` lines 469–470, the upload sentence (issue 15 step 4) |
| **`real-arduino-plan.md` Done-when 3** and the other Done-when checks in both plans. | Phases 1–2 | none named by the plans; each check passes, or its fault is fixed |

**Done when:** every record named in the table is written, and every
Done-when check passes.
