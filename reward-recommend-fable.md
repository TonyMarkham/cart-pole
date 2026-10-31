# Recommended reward for this rig

A recommendation, not a plan and not a requirement. Written 2026-10-25
from the research in `reward-research.md`; each design choice names the
source it rests on. Nothing here describes what the trainer does today.

The rig: a 1000 mm rail, a 500 mm pole, four ArUco markers (Left and
Right on the rail mounts, Pivot on the cart at the pole's hinge, End on
the pole tip). The command is the stepper's pulse rate.

Two separate things reach the trainer, and this file keeps them apart:

- **The observation** is what the policy sees and acts on: the detected
  image positions of the four markers, as a pool of the last six frames
  with their ages. Nothing else. This is what the real robot will see.
- **The reward** is the score attached to each step. The policy never
  sees how it is computed. It is computed from MuJoCo's true state of the
  simulated rig, in metres and radians, because training happens only in
  the synthetic loop and MuJoCo stands in for the physical rig.

Each numbered section gives the equation, then what every symbol is on
the rig, then the sources that aspect rests on. The whole calculation,
assembled, comes first.

> ## The complete calculation
>
> One step of the loop is: the policy issues a command, the simulator
> advances one control period, and the loop pays the policy one number,
> the reward, for that step. This section is that number's full
> calculation. Lines beginning with `#` are notes; every other line in a
> code block is part of the calculation.
>
> ```
> # ---- symbols ------------------------------------------------------
> # t        the step counter. Step t is the step being scored.
> # *        multiply.      ^2   squared.      sqrt(v)   square root of v.
> # sin(v), cos(v)   sine and cosine of an angle v given in radians.
> # exp(v)   e raised to the power v, e = 2.718...  exp(0) = 1;
> #          exp of a large negative number is close to 0, never 0.
> # ln(v)    the natural logarithm of v: the power e must be raised to
> #          to give v. ln(0.1) = -2.303.
> # |v|      the absolute value of v: v with its sign removed.
> # (p, q)   a coordinate pair: p metres sideways, q metres up. Not the
> #          interval notation (0, 1] explained under Notation below.
>
> # ---- fixed facts of this rig --------------------------------------
> L = 0.5        # pole length, metres: hinge to the End marker
> H = 0.5        # rail half-length, metres: rail centre to either end
>
> # ---- inputs, read after the simulator has advanced ----------------
> # x        cart position, metres, from the rail centre; positive when
> #          the cart is right of centre (MuJoCo)
> # a        pole lean from vertical, radians; zero is upright, positive
> #          when the tip leans right (MuJoCo)
> # c_t      the command the policy issued for step t, pulses per second
> # c_(t-1)  the command it issued for the previous step
>
> # ---- tunables, each kept in the toml ------------------------------
> # w        width of the tip reward, metres.
> #          Starting value proposed: 0.2 (section 3).
> # m        jump limit: the command change, in pulses per second, that
> #          the stepper takes without missing steps.
> #          No value proposed; it is the stepper's acceleration limit,
> #          which this file does not know (section 4).
> # b        safety buffer before the rail end, metres.
> #          No value proposed (section 6).
> # a_max    the lean, radians, past which the attempt is over.
> #          No value proposed (section 6).
>
> # ---- tip distance -------------------------------------------------
> tip_x = x + L * sin(a)          # pole tip, metres right of rail centre
> tip_y = L * cos(a)              # pole tip, metres above the rail
> d     = sqrt(tip_x^2 + (tip_y - L)^2)
> # d is the distance from the pole tip to the goal point (0, L): the
> # tip's place when the cart is centred and the pole upright.
>
> # ---- tip reward ---------------------------------------------------
> r_tip = exp(-(d / w)^2)
>
> # ---- smoothness factor --------------------------------------------
> j        = c_t - c_(t-1)
> f        = exp(-0.5 * (2.146 * j / m)^2)
> r_smooth = 0.8 + 0.2 * f
> # 2.146 is sqrt(-2 * ln(0.1)); it makes f equal exactly 0.1 when the
> # command change j equals the jump limit m (section 4).
>
> # ---- step reward --------------------------------------------------
> r = r_tip * r_smooth
> # r is the single number paid to the policy for step t. Nothing else
> # is paid. r is always in (0, 1], defined under Notation below.
>
> # ---- episode end, checked after r is paid -------------------------
> # The attempt ends, and the next step starts from a reset, if either
> # condition holds. No number is subtracted when it ends.
> end if |x| > H - b
> end if |a| > a_max
> ```
>
> Written as one expression:
>
> ```
> r = exp( -( sqrt( (x + L * sin(a))^2 + (L * cos(a) - L)^2 ) / w )^2 )
>     * ( 0.8 + 0.2 * exp( -0.5 * (2.146 * (c_t - c_(t-1)) / m)^2 ) )
> ```
>
> **Notation.** `(0, 1]` describes the range of values `r` can take; it
> is not an operation on anything. The round bracket `(0` means 0 is
> excluded: `r` can come arbitrarily close to 0 but never reach it,
> because `exp` of any finite number is positive. The square bracket
> `1]` means 1 is included: `r` is exactly 1 when the pole tip is on the
> goal point and the command did not change. So `(0, 1]` reads "greater
> than 0 and at most 1". Some papers write the same range as `]0, 1]`,
> with an outward square bracket for the excluded end.
>
> **What the trainer does with r.** Over an attempt the trainer adds up
> the per-step rewards, multiplying each later one by a factor slightly
> below 1 raised to how many steps away it is, so a reward far in the
> future counts for less than one now. That factor is the discount, and
> the weighted sum is the return. The policy is trained to make the
> return large. It never sees `r_tip`, `r_smooth`, `d`, `x` or `a`.
>
> Every line is defined and sourced in sections 1 to 6 below.

> ## 1. The state the reward reads
>
> ```
> x   cart position, metres, from the rail centre; positive to the right
> a   pole lean from vertical, radians; zero upright, positive to the right
> ```
>
> - `x`: how far the cart's hinge is from the middle of the rail. On the
>   physical rig this is a real distance whether or not anything measures
>   it. In the loop it is MuJoCo's slider joint position. The rail ends
>   are at `x = +H` and `x = -H`, with `H` = 0.5 m the rail half-length
>   (section 6).
> - `a`: how far the pole leans from straight up. In the loop it is
>   MuJoCo's hinge joint angle. The pole is `L` = 0.5 m long (section 2).
> - Neither value is ever placed in the observation. The policy sees the
>   markers only.
>
> > #### Sources
> >
> > Every rig that balanced on real hardware scored its reward from the
> > measured physical state, read by encoders, while the policy saw
> > whatever it saw: Muratore et al. on the Quanser state `[x, θ, ẋ, θ̇]`
> > ([arXiv 1907.04685](https://ar5iv.labs.arxiv.org/html/1907.04685),
> > Appendix A); the Pyrado code with the same state order
> > ([SimuRLacra quanser_cartpole.py](https://raw.githubusercontent.com/famura/SimuRLacra/master/Pyrado/pyrado/environments/pysim/quanser_cartpole.py));
> > the Lyapunov rig on `[p, α, ṗ, α̇]`
> > ([arXiv 2208.06721](https://ar5iv.labs.arxiv.org/html/2208.06721));
> > the 2026 lab cart-pole on `(x, ẋ, α, α̇)`
> > ([arXiv 2606.22145](https://arxiv.org/html/2606.22145v1)); PILCO on
> > the cart-pole state
> > ([Deisenroth 2015](https://ar5iv.labs.arxiv.org/html/1502.02860)).
> > Scoring from the simulator's state is also how dm_control's cartpole
> > reward works (`physics.cart_position()`, `physics.pole_angle_cosine()`,
> > [dm_control cartpole.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
> > That the reward need not be computable on the real robot follows
> > from the project rule that training happens only in the synthetic
> > loop (`AGENTS.md`, *The real robot*: Trainer, "none — training
> > happens only in the synthetic loop"). The user confirmed this
> > reading on 2026-10-25.

> ## 2. The tip distance
>
> ```
> L     = 0.5
> tip_x = x + L * sin(a)
> tip_y = L * cos(a)
> d     = sqrt(tip_x^2 + (tip_y - L)^2)
> ```
>
> - `L`: the pole length, 0.5 m, hinge to the End marker. A fixed fact
>   of this rig, not a tunable.
> - `x`, `a`: the cart position and pole lean from section 1.
> - `sin`, `cos`: sine and cosine of the lean angle, which is in radians.
> - `tip_x`, `tip_y`: where the top of the pole is, in metres, measured
>   from the rail centre sideways and from the rail upwards.
> - The goal point is `(0, L)`: the pole tip's place when the cart is at
>   the rail centre and the pole exactly upright, `L` straight above the
>   centre.
> - `sqrt`: square root; `^2`: squared.
> - `d`: the straight-line distance from the pole tip to the goal point,
>   in metres. Zero only when the cart is centred and the pole upright. A
>   cart off centre moves the tip sideways; a lean moves it sideways and
>   down. Either grows `d`.
>
> > #### Sources
> >
> > Both tip-distance rigs that balanced on real hardware use the
> > Euclidean distance from the pole tip to the target: the DDPG paper's
> > "d is the Cartesian Euclidean distance from the pole tip to the
> > target", the target being the tip position "when the cart is at
> > x = 0 and the pole is upright"
> > ([arXiv 2203.02230](https://ar5iv.labs.arxiv.org/html/2203.02230),
> > Eq. 9), and PILCO's "geometric distance from the state to the target
> > state" with the cart-pole target being the tip position
> > ([Deisenroth 2015](https://ar5iv.labs.arxiv.org/html/1502.02860),
> > Sec. VI). The 0.5 m pole length and the ±0.5 m rail are this rig's
> > facts (user, 2026-10-24).

> ## 3. The tip reward
>
> ```
> r_tip = exp(-(d / w)^2)
> ```
>
> - `exp`: e raised to the given power, e being 2.718. `exp(0)` is 1;
>   `exp` of a large negative number is near 0.
> - `w`: the width, in metres: the tip distance at which the score has
>   fallen to 0.37. Small makes the policy fussy about tiny errors; large
>   makes it indifferent. A tunable in the toml. Starting value 0.2 m.
> - `r_tip`: the pole-and-cart score. Always in `(0, 1]`, greater than 0
>   and at most 1 (notation defined under *The complete calculation*):
>   `exp` of a finite number is never 0, and it is exactly 1 only when
>   `d` is 0.
>
> With `w` = 0.2 m:
>
> | State | `d` (metres) | `r_tip` |
> |---|---|---|
> | upright at centre | 0 | 1.00 |
> | upright, 0.05 m off centre | 0.05 | 0.94 |
> | 10 degree lean at centre | 0.087 | 0.83 |
> | upright, 0.2 m off centre | 0.2 | 0.37 |
> | upright at the rail end | 0.5 | 0.002 |
>
> > #### Sources
> >
> > - The bell-curve shape is PILCO's saturating cost
> >   `c = 1 - exp(-d² / (2σ_c²))`, written here as a reward (`1 - c`)
> >   and with the width folded into `w`
> >   ([Deisenroth 2015](https://ar5iv.labs.arxiv.org/html/1502.02860),
> >   Sec. VI). That rig swung up and balanced a real cart-pole after
> >   17.5 s of interaction. The DDPG rig used `exp(-5 d)`, the same idea
> >   with a linear exponent
> >   ([arXiv 2203.02230](https://ar5iv.labs.arxiv.org/html/2203.02230),
> >   Eq. 9); the bell curve was chosen over it because PILCO's authors
> >   credit the flat far tail for exploration and it is the form that
> >   also learned swing-up.
> > - Bounding the reward in `(0, 1]` is Muratore et al.'s stated reason
> >   for their exponentiated quadratic: "the reward is in range ]0,1]
> >   for every time step" (`]0,1]` is the same range as `(0, 1]`, see
> >   *Notation* under *The complete calculation*)
> >   ([arXiv 1907.04685](https://ar5iv.labs.arxiv.org/html/1907.04685),
> >   Appendix A, Eq. 40), on a real Quanser cart-pole.
> > - The width value, 0.2 m, has no published source. PILCO used 0.25 m
> >   in simulation on a different rig; 0.2 m is chosen so that a 10
> >   degree lean at centre still pays 0.83 and the rail end pays
> >   nothing, read off the table above. It is a tunable for that reason.

> ## 4. The smoothness factor
>
> ```
> j        = c_t - c_(t-1)
> f        = exp(-0.5 * (2.146 * j / m)^2)
> r_smooth = 0.8 + 0.2 * f
> ```
>
> - `c_t`: the command the policy issued for step t, the step being
>   scored: the stepper pulse rate, in pulses per second.
> - `c_(t-1)`: the command it issued for the previous step.
> - `j`: the command change. A large change jerks the stepper and can
>   make it miss steps.
> - `m`: the jump limit: the command change, in pulses per second, that
>   the stepper takes without missing steps. It is the stepper's
>   acceleration limit, which this file does not know. A tunable in the
>   toml.
> - `f`: DeepMind Control's `tolerance` function written out: 1 when `j`
>   is zero, falling to exactly 0.1 when `j` reaches `m`. The constant
>   2.146 is `sqrt(-2 * ln(0.1))`, where `ln` is the natural logarithm;
>   that value is what makes `f` land on 0.1 at the limit.
> - `r_smooth`: a multiplier for gentle driving, between 0.8 and 1. The
>   floor of 0.8 means the roughest command still keeps 80 percent of
>   the score.
>
> > #### Sources
> >
> > - The function `f` is DeepMind Control's `tolerance` with its
> >   defaults: Gaussian shape, `value_at_margin = 0.1`, scale
> >   `sqrt(-2 * ln(0.1))`
> >   ([dm_control rewards.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/utils/rewards.py)).
> >   The floor is dm_control's own rescaling of its control term,
> >   `small_control = (4 + small_control) / 5`, which is `0.8 + 0.2 * f`
> >   ([dm_control cartpole.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
> >   That reward is simulation only.
> > - Penalising the *change* in command rather than its size has no
> >   cart-pole precedent. Every hardware rig in the research penalised
> >   the command's size (`aᵀRa` or `u²`) because their command was motor
> >   voltage. Here the command is a pulse rate, so a large command is a
> >   fast cart, not a hard push, and the hazard is the jump between
> >   commands. The one hardware paper that reports the hazard handled it
> >   with a low-pass filter, after an unfiltered policy switched ±10 V
> >   in under 0.05 s and broke a wheel
> >   ([arXiv 2606.22145](https://arxiv.org/html/2606.22145v1)).
> > - The floor's purpose rests on Lutter et al. on a real Quanser
> >   cart-pole: the stiff policy "applies large actions that quickly
> >   break the linear actuator's stiction, so the cart settles at the
> >   center", while the softer policy let the cart oscillate around
> >   centre
> >   ([arXiv 2110.01954](https://ar5iv.labs.arxiv.org/html/2110.01954)).
> >   A smoothness term that could zero the reward would push toward the
> >   soft policy.
> > - The jump limit `m` has no published source; it is the stepper's
> >   own acceleration limit, which this file does not know.

> ## 5. The step reward
>
> ```
> r = r_tip * r_smooth
> ```
>
> - `r`: the single score paid to the policy for this step. Always in
>   `(0, 1]`, because both factors are (notation defined under *The
>   complete calculation*).
>
> > #### Sources
> >
> > Multiplying bounded terms is dm_control's cartpole balance reward,
> > `upright * small_control * small_velocity * centered`
> > ([dm_control cartpole.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)),
> > and the dm_control paper's stated reason: terms in the unit interval
> > keep the product in the unit interval
> > ([arXiv 2006.12983](https://arxiv.org/pdf/2006.12983)). Simulation
> > only; no hardware rig in the research used a product.

> ## 6. Episode end
>
> ```
> H = 0.5
> end if |x| > H - b
> end if |a| > a_max
> ```
>
> - `H`: the rail half-length, 0.5 m, rail centre to either end. A fixed
>   fact of this rig, not a tunable.
> - `x`, `a`: the cart position and pole lean from section 1.
> - `|x|`, `|a|`: their absolute values, the sign removed, so both rail
>   ends and both lean directions are treated alike.
> - `b`: the safety buffer before the rail end, in metres. A tunable in
>   the toml.
> - `a_max`: the lean past which the attempt is over, in radians. A
>   tunable in the toml.
> - Nothing is subtracted at the end. Every step pays a positive score,
>   so ending early already forfeits all the score the attempt would
>   have earned.
>
> > #### Sources
> >
> > - Ending the attempt at a rail buffer and a fall angle follows the
> >   Pyrado stabilisation task: fail at `|θ - π| > 15°`, cart bounds
> >   `±(l_rail/2 - 0.05 m)` with a 0.05 m buffer
> >   ([SimuRLacra quanser_cartpole.py](https://raw.githubusercontent.com/famura/SimuRLacra/master/Pyrado/pyrado/environments/pysim/quanser_cartpole.py)),
> >   and the DDPG rig's terminal at `|x| > 0.34 m`
> >   ([arXiv 2203.02230](https://ar5iv.labs.arxiv.org/html/2203.02230)).
> > - Not subtracting a penalty at the end is the weaker-supported
> >   choice. Two hardware rigs did add one: 100 past 0.23 m
> >   ([arXiv 2606.22145](https://arxiv.org/html/2606.22145v1)) and 5000
> >   past 0.3 m
> >   ([arXiv 2208.06721](https://ar5iv.labs.arxiv.org/html/2208.06721)),
> >   both on top of a *negative* quadratic reward, where ending early
> >   would otherwise be cheaper than continuing. With a positive bounded
> >   reward that incentive does not exist; dm_control's cartpole has no
> >   termination at all and relies on this
> >   ([dm_control cartpole.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
> >   The analysis of survival-only versus death-only terminal rewards
> >   supports keeping one mechanism, not both
> >   ([NeurIPS 2026](https://neurips.cc/virtual/2026/161771)). If the
> >   policy is seen running the cart to the buffer anyway, adding the
> >   penalty is the published fallback.
> > - `b` and `a_max` have no published values for this rig. Pyrado's
> >   15° and 50 mm are the closest precedents.

> ## 7. What else MuJoCo's state is for
>
> ```
> reward       yes   sections 1 to 6
> measurement  yes   ground truth for judging a run
> episode setup yes  resets and the end conditions in section 6
> observation  NEVER
> ```
>
> - **Measurement.** The true state is the ground truth for what a
>   trained policy actually does: how far off centre the cart sits once
>   balanced, how long it takes to settle, how often it reaches the
>   buffer, how much the command jumps. None of the hardware papers
>   report these numbers because their rigs made them hard to get; here
>   they are free, and they are what `trainer-observations.md` exists to
>   record.
> - **Episode setup.** Resets to a chosen cart position and lean, and the
>   end conditions, read the state directly.
> - **Observation: never.** The policy acts on the six frames of marker
>   positions and nothing else. The moment true state reaches the
>   policy's input, the policy is no longer one the real robot can run,
>   because the real robot has no such input.
>
> > #### Sources
> >
> > The observation rule is the project's, not the literature's:
> > `AGENTS.md`, *Not a toy*: the policy "sees only what the robot will
> > see — detected markers, with the detector's delay — never the
> > simulator's true state". The measurement use follows from the gap
> > noted in `reward-research.md` section 7: no hardware paper reports a
> > numeric steady-state cart offset. The user confirmed the three uses
> > and the one prohibition on 2026-10-25.

## Why this one

- The rig's geometry sets the trade between centring and uprightness,
  with no hand-tuned weight. The pole is as long as half the rail, so a
  10 degree lean and a 0.087 m offset cost the same, and the rail end
  costs the same as an unrecoverable lean.
  *Source:* the published hand-tuned angle-to-position ratios run from
  1 : 1 ([Marco 2017](https://ar5iv.labs.arxiv.org/html/1703.01250),
  Eq. 13) through 2 : 1 ([Pyrado](https://raw.githubusercontent.com/famura/SimuRLacra/master/Pyrado/pyrado/environments/pysim/quanser_cartpole.py))
  and 5 : 1 ([arXiv 2208.06721](https://ar5iv.labs.arxiv.org/html/2208.06721))
  to 100 : 1 ([arXiv 1907.04685](https://ar5iv.labs.arxiv.org/html/1907.04685);
  [arXiv 2606.22145](https://arxiv.org/html/2606.22145v1)), with no
  agreement; and Quanser's own quadratic-reward policy settled about 15°
  off its arm target on the real Qube
  ([Quanser blog](https://www.quanser.com/blog/artificial-intelligence/using-the-reinforcement-learning-toolbox-to-balance-the-qube-servo-3-inverted-pendulum/)).
  The tip-distance form removes the choice.
- The reward scores the physics, not the camera. Cart offset in metres
  and lean in radians mean the same thing in Unity and on the rig; image
  pixels do not, since the scale varies with camera placement, roll and
  lens distortion.
  *Source:* every hardware rig in the research scored from measured
  physical state (section 1 above). An earlier draft of this file scored
  from marker pixels; the user identified the weakness on 2026-10-25 and
  it was replaced.
- Two real cart-poles balanced on this form.
  *Source:* PILCO, real cart-pole, 17.5 s of interaction
  ([Deisenroth 2015](https://ar5iv.labs.arxiv.org/html/1502.02860));
  cloud-edge DDPG, real Quanser pendulum via a Raspberry Pi
  ([arXiv 2203.02230](https://ar5iv.labs.arxiv.org/html/2203.02230)).
  Neither reports a numeric cart offset; both show centring in plots.
- The flat tail far from the goal is what let the PILCO rig learn
  swing-up as well as balance, so the form does not lock the project
  into balance-only.
  *Source:* PILCO's authors state the saturation "encourages exploration
  when predicted states are far from the target and exploitation when
  they are near it"
  ([Deisenroth 2015](https://ar5iv.labs.arxiv.org/html/1502.02860),
  Sec. VI), and their hardware result was a swing-up plus balance.

## Open items that could change it

- The control period and the stepper's limits, which set `m` and `b`.
- Whether attempts start with the pole up or hanging. For swing-up, `w`
  would be wider.
- Where in the loop the reward is computed (the orchestrator holds the
  MuJoCo state; the trainer receives the score). The design diagram
  decides; this file was written without reading it.
- The reward run 2 used, which was not read. This is not a comparison
  against it.
