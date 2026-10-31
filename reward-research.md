# Reward research: balancing the pole with the cart near the rail centre

Research, not a plan. This file answers one question: **what reward did
the cart-pole policies that balanced on real hardware use, and how did
each one make the cart stay near the centre of the rail?** Every formula
and weight below is quoted from the paper or the released code it came
from, with a link beside it. Where a mapping is inferred rather than
stated, the text says so. Nothing here describes what this repo's trainer
does today; that is verified against the trainer's sources, never against
this file.

Written 2026-10-24 from the sources listed at the end, replacing an
earlier version of this file that was not source-verified.

## 1. The finding in one paragraph

Every published policy that balanced a *real* cart-pole near the rail
centre put the cart position into a **dense, bounded** per-step reward,
weighted explicitly against the pole angle. None of them relied on the
Gymnasium-style signal of "+1 per step alive, terminate at the rail end":
that signal is indifferent to where on the rail the cart sits, so the
policy it produces is too. Three reward families appear, and all three
have worked on hardware: an **exponentiated quadratic** cost, a
**saturating pole-tip-distance** cost, and a **product of bounded
tolerance terms**.

## 2. Vocabulary (for a reader new to RL)

- **Dense reward**: a non-zero signal every step, so the policy is told
  at every instant how well it is doing. The opposite is a *sparse*
  reward, paid only at a goal or a failure.
- **Bounded reward**: every step's reward lies in a fixed range, here
  greater than 0 and at most 1. Then a longer episode is always worth
  more than a shorter one, and the policy can never profit from ending
  an episode early. Papers write such a range in interval notation:
  `(0, 1]` or `]0, 1]` both mean 0 excluded, 1 included; a round bracket
  or an outward square bracket excludes the end value, an inward square
  bracket includes it.
- **Quadratic cost**: a penalty that grows with the square of the error,
  `x²`. Doubling the error quadruples the cost. LQR (the classical
  optimal controller for this rig) minimises exactly such a sum.
- **Exponentiated quadratic**: `r = exp(-cost)`. It turns an unbounded
  cost into a bounded reward in (0, 1]: zero cost pays 1, large cost pays
  nearly 0. Near the goal it behaves like the quadratic; far away it
  flattens out.
- **Saturating cost**: `c = 1 - exp(-d²/2σ²)`. The mirror image of the
  above: near the goal it is quadratic, far away it saturates at 1. `σ`
  (the *width*) sets how far "near" reaches.
- **Tolerance term** (DeepMind Control's name): a function that is 1
  inside a target interval and falls off smoothly to a chosen value
  (default 0.1) at a chosen distance (the *margin*) outside it.
- **State vector** on a cart-pole: `[x, θ, ẋ, θ̇]` = cart position, pole
  angle, cart velocity, pole angular velocity. A `Q` matrix weights the
  state terms of a quadratic cost; `R` weights the action term.

## 3. The hardware successes, one by one

### 3.1 Muratore et al., Quanser linear pendulum, PPO + domain randomization

Source: [arXiv 1907.04685](https://ar5iv.labs.arxiv.org/html/1907.04685),
Appendix A, Eq. 40 and Table III.

- Reward: `r(s, a) = exp(-(sᵀ Q s + aᵀ R a))`, so `r` is greater than 0
  and at most 1 every step. The paper's stated reason is only that
  consequence.
- `Q = diag(10, 1000, 0.05, 0.005)`, `R = 1e-4` (the action is motor
  voltage, 1-D).
- The paper does not say which entry is which state. The same group's
  released code (3.2 below) orders the Quanser state `[x, θ, ẋ, θ̇]`; read
  that way, **pole angle is weighted 100× cart position**, and the two
  velocities are 3–5 orders of magnitude below the angle. This mapping is
  inferred from the code, not stated in the paper.
- Hardware: Quanser linear inverted pendulum (cart on a rail, servo
  motor, rack and pinion). Each real episode starts with the cart centred,
  an energy-based controller swings the pole up, and the learned policy
  then balances for 4000 steps (8 s).
- Result: policies trained in randomized simulation transferred directly
  in most cases; the policy trained on the nominal (un-randomized) model
  "failed to transfer in all trials on the Cart-Pole". The authors blame
  unmodelled rack-and-pinion effects. No numeric cart offset is reported.

### 3.2 Pyrado `QCartPoleStabSim`: the released code behind 3.1

Source: [SimuRLacra, `quanser_cartpole.py`](https://raw.githubusercontent.com/famura/SimuRLacra/master/Pyrado/pyrado/environments/pysim/quanser_cartpole.py).

- Stabilisation task: a plain quadratic error cost (`QuadrErrRewFcn`)
  wrapped in a final-state reward, target state `[0, π, 0, 0]`.
- `Q = diag(5, 10, 0.01, 0.01)` on `[x, θ, ẋ, θ̇]`, `R = 1e-3` on voltage.
  **Angle : position = 2 : 1** here; velocities 1000× lower.
- Failure when `|θ - π| > 15°`. State bounds: `|x| < l_rail/2 - 0.05 m`
  with `l_rail = 0.814 m`, so about ±0.357 m.
- Initial state: `x, ẋ` within ±0.02, `θ` within π ± 8°, `θ̇` within ±5°.
- Swing-up task, for contrast: `ExpQuadrErrRewFcn` with
  `Q = diag(0.3, 0.5, 0.005, 0.001)`, `R = 1e-3`, no final-state wrapper.
  Position and angle are weighted almost equally for swing-up.
- No comment in the code justifies the weights.

### 3.3 Lutter et al., Quanser cart-pole, continuous fitted value iteration (cFVI, rFVI)

Source: [arXiv 2110.01954](https://ar5iv.labs.arxiv.org/html/2110.01954).

- Reward is separable: `r(x, u) = q(x) - g(u)`, with `q` a negative
  distance to the goal state (negative definite, zero at the goal) and `g`
  a strictly convex action cost. The convex `g` is chosen so the optimal
  policy saturates smoothly instead of switching bang-bang (Table I lists
  quadratic, logistic, atan, tanh). The numeric state weights for the
  cart-pole are not in the text read.
- Hardware: a voltage-controlled Quanser cart-pole. Physical success
  rates (Table III): DP rFVI 100 %, RTDP cFVI 100 %, DP cFVI 73.3 %.
- **The one direct statement about centring in the whole literature
  read:** the stiffer rFVI policy "applies large actions that quickly
  break the linear actuator's stiction, so the cart settles at the
  center", while the softer cFVI policy lets the cart oscillate around
  the centre, because the pole has to fall far enough before the
  commanded correction overcomes static friction. Same cost, different
  policy stiffness, different centring.

### 3.4 Zero-shot sim-to-real swing-up and stabilisation (2026)

Source: [arXiv 2606.22145](https://arxiv.org/html/2606.22145v1).

- Two controllers, switched in Simulink.
- Swing-up (TD3):
  `R = -0.1 (x/0.25)² - 0.4 (ẋ/0.25)² - 10 (α/π)² - 0.1 (α̇/π)² - 0.01 u² - 100·B`,
  `B = 1` when `|x| > 0.23 m`, else 0. Position is **normalised by the
  rail half-width 0.25 m**, angle by π, before weighting; then angle is
  weighted 100× position, and the velocity terms are 4× (cart) and 1×
  (pole) the position weight.
- Stabiliser (REINFORCE): an indicator reward, 1 when `|x| < 0.2 m` and
  `|α| < 0.2 rad`, else 0. Cart position is in the success condition
  itself. Episodes up to 2000 steps of 0.01 s (20 s), γ = 0.99.
  Training success criterion: balance 20 s in 95 % of episodes.
- Observations: swing-up `(x, ẋ, sin α, cos α, α̇)`, stabiliser
  `(x, ẋ, α, α̇)`.
- Hardware: lab cart-pole, DC motor, ±10 V, cart 0.94 kg, pole 0.23 kg,
  0.3302 m.
- Smoothness is **not** in the reward. A first-order low-pass filter,
  τ = 0.3 s, sits on the policy output. It was added after the first
  unfiltered policy switched ±10 V in under 0.05 s (above 750 V/s) and
  broke a cart wheel within 2 s. The authors call action filtering
  "essential to protect hardware".
- Result: the stabiliser held the pole after manual taps of about 14° in
  five runs. The combined controller reached upright in 15–40 s and
  recovered from taps, described as "not very consistent". No numeric
  cart drift is reported.

### 3.5 Lyapunov-design fine-tuning, Quanser Linear Servo, SAC

Source: [arXiv 2208.06721](https://ar5iv.labs.arxiv.org/html/2208.06721).

- Reward: `r = -0.1 (5α² + p² + 0.05 u²) - 5000·1[|p| ≥ 0.3]`, with `p`
  the cart position in metres, `α` the pole angle, `u` the voltage
  (|u| < 10 V). **Angle : position = 5 : 1.** No velocity terms. A
  one-off penalty of 5000 when the cart passes 0.3 m, with no termination
  described; the rail bound lives only in the reward.
- Hardware: Quanser Linear Servo Base Unit with a 60 cm pendulum.
  Simulation at 100 Hz, hardware fine-tuning at 500 Hz, 10 s episodes.
- Validation success: within 10 s, hold `|α| < 0.12 rad`,
  `|α̇| < 0.3 rad/s` and `|p| < 0.3 m` throughout.
- Result: the simulation policy failed on hardware outright; after two
  10 s fine-tuning episodes with a reshaped reward (the simulation value
  function used as a Lyapunov term, γ = 0), the system swung up and
  balanced with the cart in bounds. The paper claims "in bounds", not
  "centred".

### 3.6 Cloud-edge DDPG on a Quanser pendulum through a Raspberry Pi

Source: [arXiv 2203.02230](https://ar5iv.labs.arxiv.org/html/2203.02230), Eq. 9.

- Reward: `R = exp(-5 d) - 0.1 a² - 20·1[terminal]`, where `d` is the
  Euclidean distance from the **pole tip** to the tip's position when the
  cart is at `x = 0` and the pole upright.
- The tip target couples centring and uprightness in a single term: a
  cart parked at the rail end with a perfect pole still pays the full
  rail offset as distance. The exponential bounds the term to (0, 1] and,
  the authors say, its steep gradient near zero helps the agent settle
  exactly on target.
- Terminal at `|x| > 0.34 m` or `|θ̇| ≥ 20 rad/s`; reset after 1000 steps
  or 100 consecutive on-target steps (on-target = tip within 0.05 m).
- Hardware: Quanser linear inverted pendulum; the actor runs at 30 Hz on
  a Raspberry Pi 4B. Algorithm: DDPG with TD3-style target smoothing.
- No cart-centring measurement is reported.

### 3.7 PILCO, a real cart-pole learned from 17.5 s of data

Source: [Deisenroth, Fox, Rasmussen 2015](https://ar5iv.labs.arxiv.org/html/1502.02860),
Sec. VI and the cart-pole experiment.

- Cost: `c(x) = 1 - exp(-d²/(2σ_c²))`, `d` = Euclidean distance from the
  pendulum tip to the target tip position, `σ_c = 0.25 m` in simulation
  (the hardware value is not in the text read). The same tip-target
  trick as 3.6, so the cart position enters through the tip. The cost
  "requires the pendulum tip to move above horizontal to avoid full
  cost".
- The authors credit the saturation (cost ≈ 1 everywhere far from the
  goal) for exploration: far from the goal all states look equally bad,
  so the model-based optimiser is not misled by small differences.
- Hardware: the controller swung up and balanced the real cart-pole
  after 17.5 s of total interaction. Simulation used a 10 Hz control
  signal and 2.5 s episodes; the hardware test shown is 20 s. Cart
  position at the end is not reported numerically.

### 3.8 Marco et al., Bayesian optimisation on a Quanser pendulum

Source: [arXiv 1703.01250](https://ar5iv.labs.arxiv.org/html/1703.01250), Eq. 13.

- Cost over a K-step horizon: `J = (1/K) Σ [s² + ψ² + ṡ² + 0.1 ψ̇² + 10^-1.5 u²]`,
  cart position `s`, pole angle `ψ`, voltage `u`. **Angle : position =
  1 : 1**, cart velocity also 1, pole velocity 0.1, voltage ≈ 0.032.
- This is an LQR cost evaluated on rollouts, tuned by Bayesian
  optimisation, not a learned neural policy. Hardware: Quanser Linear
  Inverted Pendulum via Matlab/Simulink. No centring analysis.

### 3.9 MC-PILCO (simulation cart-pole, real Furuta pendulum)

Source: [arXiv 2101.12115](https://ar5iv.labs.arxiv.org/html/2101.12115), Eq. 19.

- Cost: `c = 1 - exp(-((|θ| - π)/l_θ)² - (p/l_p)²)`, best setting
  `l_θ = 3`, `l_p = 1` (length scales; smaller is more selective). Cart
  position `p` has its own explicit term with its own length scale, as
  an alternative to the tip-distance coupling of 3.6 and 3.7. The
  absolute value on θ accepts a swing-up to either +π or -π.
- Listed for the cost form only: its cart-pole results are simulated.

## 4. What the standard benchmark rewards do about centring

These are reference tasks (`gymnasium-research.md`), not the task, and
they show why a benchmark reward does not centre the cart.

- **Gymnasium `CartPole-v1`**: +1 per step including the terminal step,
  no position term; the episode ends at `|x| > 2.4` or `|θ| > 12°`
  ([docs](https://gymnasium.farama.org/environments/classic_control/cart_pole/)).
- **Gymnasium `InvertedPendulum-v5`**: +1 per step, terminate at
  `|θ| > 0.2 rad`, truncate at 1000 steps; no rail limit in the reward
  at all ([docs](https://gymnasium.farama.org/environments/mujoco/inverted_pendulum/)).
- **Isaac Lab cartpole**: `+1·alive`, `-2·terminated`, `-1·θ²`,
  `-0.01·|ẋ|`, `-0.005·|θ̇|`; terminate at `|x| > 3 m`. No position term;
  the cart-velocity penalty limits drift speed but never pulls the cart
  back ([Isaac Lab how-to](https://isaac-sim.github.io/IsaacLab/main/source/how-to/create_manager_rl_env.html)).
- **DeepMind Control `cartpole balance`**, the product-of-tolerances
  reference ([cartpole.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py),
  [rewards.py](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/utils/rewards.py)):

  ```
  upright        = (cos θ + 1) / 2                                             # [0, 1]
  centered       = tolerance(x, margin=2);  centered = (1 + centered) / 2      # [0.5, 1]
  small_control  = tolerance(u, margin=1, value_at_margin=0, sigmoid='quadratic')
                   small_control = (4 + small_control) / 5                     # [0.8, 1]
  small_velocity = tolerance(θ̇, margin=5).min();  small_velocity = (1 + small_velocity) / 2   # [0.5, 1]
  reward         = upright * small_control * small_velocity * centered
  ```

  `tolerance(x, bounds=(0, 0), margin, sigmoid='gaussian', value_at_margin=0.1)`
  is 1 inside the bounds and falls as a Gaussian to 0.1 at the margin
  (gaussian: `exp(-0.5 (d·scale)²)` with `scale = sqrt(-2 ln 0.1)`;
  quadratic: `1 - (d·scale)²`, clipped at 0). Each secondary term is
  rescaled into a floor above zero so that no single factor can zero
  the product: the worst `centered` halves the reward, the worst
  `small_control` takes a fifth. The sparse variant instead requires
  `|x| < 0.25` and `cos θ > 0.995`. The task's stated goal is to balance
  "in the middle of the trail". This is simulation only, but it is the
  reward most current benchmark work (MuJoCo Playground among others)
  inherits.
- **Quanser's own Qube RL example** (rotary pendulum, not linear): a
  quadratic reward on arm angle, pendulum angle and voltage, chosen
  because quadratic rewards "are easier to tune and increase the
  likelihood of training a successful policy". On the real Qube the arm
  still settled about 15° off its target, the recurring failure mode of
  a weak position weight ([Quanser blog](https://www.quanser.com/blog/artificial-intelligence/using-the-reinforcement-learning-toolbox-to-balance-the-qube-servo-3-inverted-pendulum/)).
- **A 2025 simulation study** zeroes the +1 reward whenever `|x| > 0.1 m`
  without terminating, and reports that this "encourages the cart to
  stay near the origin instead of anywhere in the allowed range"
  ([arXiv 2507.00268](https://ar5iv.labs.arxiv.org/html/2507.00268)).
  A binary, non-terminating position gate; simulation only.

## 5. Published angle-to-position weight ratios

| Source | Form | Angle : position | Velocity weights | Action weight |
|---|---|---|---|---|
| Muratore 2019 (3.1) | exp-quadratic | 100 : 1 (inferred order) | 0.05, 0.005 | 1e-4 |
| Pyrado stab (3.2) | quadratic | 2 : 1 | 0.01, 0.01 | 1e-3 |
| Pyrado swing-up (3.2) | exp-quadratic | 1.7 : 1 | 0.005, 0.001 | 1e-3 |
| Zero-shot 2026 swing-up (3.4) | quadratic, normalised | 100 : 1 after normalising x by 0.25 m and α by π | 4×, 1× the position weight | 0.01 |
| Zero-shot 2026 stabiliser (3.4) | indicator | gate: 0.2 rad and 0.2 m | none | none |
| Lyapunov 2022 (3.5) | quadratic + rail penalty | 5 : 1 | none | 0.005 |
| Marco 2017 (3.8) | LQR | 1 : 1 | 1, 0.1 | 0.032 |
| Cloud-edge DDPG (3.6), PILCO (3.7) | tip distance | set by geometry: pole length vs rail offset | none | 0.1 (DDPG), none (PILCO) |
| dm_control balance | tolerance product | upright ∈ [0, 1] vs centered ∈ [0.5, 1], margin 2 m | θ̇ margin 5 | quadratic, margin 1 |

The range runs from 1 : 1 (classical LQR cost) to 100 : 1. The two most
recent hardware papers settled on 5 : 1 and 100 : 1 (after normalising).
The tip-distance designs avoid the choice: with a pole of length `l` and
a cart offset `x`, the tip distance for an upright pole is exactly `|x|`,
and for a lean of `α` at centre it is about `l·α`, so the geometry sets
the exchange rate.

## 6. Lessons the sources support

Each lesson names the sources it rests on. These are observations about
what worked elsewhere, not directions for this repo.

1. **Position is weighted explicitly, or coupled through the tip.** All
   eight hardware designs (3.1–3.8) have the cart position in the dense
   reward: six as a weighted term, two through the pole-tip target.
   None centres the cart by termination alone.
2. **Normalise before weighting.** 3.4 divides `x` by the rail
   half-width and `α` by π so the weights compare like with like. 3.9's
   length scales do the same job. Without it, a weight ratio depends on
   whether `x` is in metres or millimetres.
3. **Keep the per-step reward positive and bounded.** The exponentiated
   quadratic (3.1, 3.6), the saturating cost (3.7, 3.9) and the
   tolerance product (dm_control) all guarantee a longer episode is
   worth more. A pure negative-penalty reward with termination lets the
   policy learn that failing early is cheaper than a long, imperfect
   balance; a 2026 analysis of survival-only vs death-only terminal
   rewards shows the equivalence between them breaks as soon as the
   reward is discounted or shaped
   ([NeurIPS 2026](https://neurips.cc/virtual/2026/161771)).
4. **A dense centring term plus a one-off rail penalty is the common
   hardware pattern.** 3.4 adds 100 past 0.23 m and 3.5 adds 5000 past
   0.3 m, on top of the quadratic that does the steering. The dense term
   centres; the penalty guards the end stops.
5. **Stiction decides whether centring happens on the rig.** 3.3 is the
   direct evidence: identical cost, the stiff policy centred the cart,
   the soft policy let it wander because small corrections never broke
   static friction. A reward that punishes every small command works
   against centring on a sticky drive.
6. **Smooth the action, in the reward or after it.** Every hardware
   paper handles it somewhere: `aᵀRa` (3.1, 3.2, 3.5, 3.6), a convex
   action cost shaping the policy (3.3), or a low-pass filter on the
   command (3.4, after breaking a wheel). dm_control's `small_control`
   is the tolerance-product version.
7. **Velocity terms are small.** Where present, cart and pole velocity
   weights sit 1 to 3 orders of magnitude below the angle weight (3.1,
   3.2, 3.8). They damp oscillation around centre without competing
   with the main objective. 3.5 omits them entirely and still balanced.

## 7. Gaps

- No hardware paper reports a numeric steady-state cart offset. "Near
  centre" is shown in plots and described in words.
- The PILCO hardware cost width and the Lutter cart-pole state weights
  are not in the texts read.
- The Muratore 2019 Q-to-state mapping is inferred from the released
  code's state order, not stated in the paper.
- No source trains against a detected-marker observation with a
  detector delay; all observe the state from encoders. The delay's
  effect on the centring term is unstudied in this set.

## Sources

- Muratore et al., "Assessing Transferability from Simulation to Reality
  for Reinforcement Learning", arXiv 1907.04685 —
  https://ar5iv.labs.arxiv.org/html/1907.04685
- SimuRLacra / Pyrado, `quanser_cartpole.py` —
  https://raw.githubusercontent.com/famura/SimuRLacra/master/Pyrado/pyrado/environments/pysim/quanser_cartpole.py
- Lutter et al., "Continuous-Time Fitted Value Iteration for Robust
  Policies", arXiv 2110.01954 — https://ar5iv.labs.arxiv.org/html/2110.01954
- "Zero-shot Transfer of Reinforcement Learning Control Policies for the
  Swing-Up and Stabilization of a Cart-Pole System", arXiv 2606.22145 —
  https://arxiv.org/html/2606.22145v1
- "Lyapunov Design for Robust and Efficient Robotic Reinforcement
  Learning", arXiv 2208.06721 — https://ar5iv.labs.arxiv.org/html/2208.06721
- "Cloud-Edge Training Architecture for Sim-to-Real Deep Reinforcement
  Learning", arXiv 2203.02230 — https://ar5iv.labs.arxiv.org/html/2203.02230
- Deisenroth, Fox, Rasmussen, "Gaussian Processes for Data-Efficient
  Learning in Robotics and Control", arXiv 1502.02860 —
  https://ar5iv.labs.arxiv.org/html/1502.02860
- Marco et al., "Virtual vs. Real: Trading Off Simulations and Physical
  Experiments in Reinforcement Learning with Bayesian Optimization",
  arXiv 1703.01250 — https://ar5iv.labs.arxiv.org/html/1703.01250
- Amadio et al., "Model-Based Policy Search Using Monte Carlo Gradient
  Estimation with Real Systems Application" (MC-PILCO), arXiv 2101.12115
  — https://ar5iv.labs.arxiv.org/html/2101.12115
- dm_control `suite/cartpole.py` —
  https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py
- dm_control `utils/rewards.py` —
  https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/utils/rewards.py
- Isaac Lab, "Creating a Manager-Based RL Environment" (cartpole
  RewardsCfg) —
  https://isaac-sim.github.io/IsaacLab/main/source/how-to/create_manager_rl_env.html
- Gymnasium `CartPole-v1` —
  https://gymnasium.farama.org/environments/classic_control/cart_pole/
- Gymnasium `InvertedPendulum-v5` —
  https://gymnasium.farama.org/environments/mujoco/inverted_pendulum/
- Quanser, "Using the Reinforcement Learning Toolbox to balance the
  QUBE-Servo 3 inverted pendulum" —
  https://www.quanser.com/blog/artificial-intelligence/using-the-reinforcement-learning-toolbox-to-balance-the-qube-servo-3-inverted-pendulum/
- Fivel, Rudman, Cohen, "Control-Optimized Deep Reinforcement Learning
  for Artificially Intelligent Autonomous Systems", arXiv 2507.00268 —
  https://ar5iv.labs.arxiv.org/html/2507.00268
- "Between Life and Death: Examining Failure Modes of Sparse Reward
  Designs in Healthcare RL", NeurIPS 2026 —
  https://neurips.cc/virtual/2026/161771
