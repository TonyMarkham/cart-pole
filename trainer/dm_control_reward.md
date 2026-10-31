# dm_control's cart-pole reward, explained simply

This is DeepMind's cart-pole reward, the one `trainer/reward-plan.md`
borrows its shape from (its notes 4, 19, 22, 23 and 25), explained the
same way. It is read from the code itself, the dm_control checkout in
`submodules/dm_control` at commit `e94f1e9`[^1]. It is not this rig's
reward: *How it differs from this rig's reward*, at the end, lists the
differences.

> ## Where the reward's numbers come from
>
> dm_control's cart-pole is a MuJoCo model too, but a made-up one, not a
> rig's CAD: a 1 kg box of a cart on a 3.6 m rail, 1 m above a floor,
> with a 1 m pole of 0.1 kg hinged at the cart's centre[^3]. The pole is
> drawn as a capsule 0.045 m thick. A motor pushes the cart along the
> rail. The rail runs along `x`; the pole swings in the `x`–`z` plane,
> about a hinge axis along `y`; `z` is up.
>
> **Every number the reward uses comes from MuJoCo's state**, read at
> the end of each step[^1]:
>
> - **The cart's position**, `x`, in metres from the middle of the rail:
>   MuJoCo's `qpos` for the `slider` joint. Positive is one way along the
>   rail, negative the other; the rail ends at ±1.8 m, and the joint is
>   limited there, so the cart can go no further.
> - **The pole's angle**, through its cosine, `cos θ`: MuJoCo's
>   rotation matrix for the pole's body, `xmat`, has an entry, `zz`, that
>   is the cosine of the angle between the pole's own up and the world's
>   up[^7]. `θ` is 0 with the pole straight up, 90° sideways, 180°
>   hanging. The reward never sees `θ` itself, only `cos θ`: 1 straight
>   up, 0 sideways, −1 hanging.
> - **The pole's turning speed**, `ω` ("omega"), in radians per second:
>   MuJoCo's `qvel` for the hinge[^10]. 0 when the pole is still,
>   negative when it turns the other way. 6.28 rad/s is one full turn a
>   second.
> - **The push**, `u`: the command the policy gave the motor on this
>   step, MuJoCo's `ctrl`[^10]. It runs from −1 (full push one way) to
>   +1 (full push the other); the motor's `gear` of 10 makes that a
>   force of −10 N to +10 N on the cart[^3].
>
> These are ground truth, with no measurement error. In dm_control the
> policy sees them too: its observation is the cart's position, the
> pole's `cos θ` and `sin θ`, and every joint's speed[^1]. That is the
> first difference from this rig (*How it differs*).
>
> The model allows more than one pole (`two_poles`, `three_poles`).
> This explanation is for the one-pole tasks, where the code's `.mean()`,
> `.min()` and `[0]` over poles and motors each pick the single value
> there is.

> ## 0 The Reward
> ```text
> r = upright × small_control × small_velocity × centered
> ```
>
> After every step the policy gets a score, `r`, between 0 and 1. The
> score is four smaller scores multiplied: one for the pole standing up
> (`upright`), one for pushing gently (`small_control`), one for the pole
> turning slowly (`small_velocity`), and one for the cart being near the
> middle of the rail (`centered`). Multiplying means all four matter: a
> bad mark on any of them drags the whole score down.
>
> With the cart in the middle, the pole straight up and still, no push:
> 1. The same with the cart 1 m from the middle: 0.781. With a full push:
> 0.8. With the pole passing the top at 5 rad/s: 0.55. All three at once:
> 0.344. Pole leaning 10°, still, centred: 0.992. Pole sticking out
> sideways, still: 0.5. Pole hanging, still: 0, whatever the rest.
>
> ```text
> situation                              upright  small_control  small_velocity  centered  r
> up, still, centred, no push            1        1              1               1         1
> up, still, cart 1 m out                1        1              1               0.781     0.781
> up, still, centred, full push          1        0.8            1               1         0.8
> passing the top at 5 rad/s, centred    1        1              0.55            1         0.55
> all three of the above                 1        0.8            0.55            0.781     0.344
> leaning 10°, still, centred            0.992    1              1               1         0.992
> sideways, still, centred               0.5      1              1               1         0.5
> hanging, still, centred                0        1              1               1         0
> hanging, 10 rad/s, 1.8 m out, pushing  0        0.8            0.500           0.577     0
> ```
>
>> ### Reasoning
>>
>> The score obeys `0 ≤ r ≤ 1`: the Control Suite keeps every reward "in
>> the unit interval"[^4]. Each of the four scores obeys it too, and
>> multiplying numbers between 0 and 1 gives a number between 0 and 1.
>>
>> Three of the four can never reach 0. `small_control` is never below
>> 0.8 (2.2), `small_velocity` never below 0.5 (3.2) and `centered` never
>> below 0.5 (4.2): each is lifted off 0 by a floor. So between them
>> they can take at most 1 − 0.8 × 0.5 × 0.5 = 0.8 of the score, and a
>> step always keeps at least a fifth of whatever `upright` pays.
>>
>> `upright` has no floor. It is 0 with the pole hanging straight down,
>> and then `r` is 0, however gently the cart is pushed and wherever it
>> is. That is a design choice the other three avoid: at the bottom of
>> the swing the only thing that pays is the pole rising, and the cart's
>> position, the push and the speed are all free there, so a swing-up can
>> run the cart out and whip the pole round at no cost. The three floored
>> scores are nudges, as this rig's `r_cart`, `r_reach`, `r_target_speed`
>> and `r_speed` are (`reward-plan.md`, sections 2 to 5): they can only
>> cost so much, and `upright` decides most of what a step earns.
>>
>> The same four factors, with the pole's angle as the main score and the
>> cart, the push and the speed as floored nudges, serve two tasks: in
>> `balance` the pole starts near the top and must stay there; in
>> `swingup` it starts hanging and must be swung up first[^5]. Nothing in
>> the reward changes between them (section 6).
>
>> ### 1 `upright`
>> ```text
>> upright = (cos θ + 1) / 2
>> ```
>>
>> The "stand the pole up" score. 1 with the pole straight up, 0.5
>> sideways, 0 hanging, and every angle between pays something between.
>> It depends on the pole's angle alone: a pole straight up scores 1
>> wherever the cart is, as this rig's moving star does (`reward-plan.md`,
>> section 1).
>>
>> ```text
>> pole from straight up    cos θ     upright
>> 0°                       1         1
>> 5.73°                    0.995     0.9975
>> 10°                      0.985     0.992
>> 20°                      0.940     0.970
>> 45°                      0.707     0.854
>> 90° (sideways)           0         0.5
>> 135°                     −0.707    0.146
>> 180° (hanging)           −1        0
>> ```
>>
>>> ### 1.1 `cos θ`
>>>
>>> How much of the pole's length points up: the cosine of its angle
>>> from straight up. For a pole 1 m long, `cos θ` is the height of its
>>> tip above the hinge, in metres: 1 m straight up, 0 sideways, −1 m
>>> (1 m below) hanging. It comes straight from MuJoCo's rotation matrix
>>> for the pole's body, the `zz` entry of `xmat`, which is exactly that
>>> cosine[^7]. Left and right leans score the same, because
>>> `cos(−θ) = cos(θ)`.
>>>
>>> So `upright` is the tip's height above the hinge, in metres, moved
>>> from the range −1 to 1 into the range 0 to 1. The gain for each
>>> degree is smallest near the top: from 0° to 10° it loses 0.008, from
>>> 80° to 90° it loses 0.087. A pole nearly up is paid nearly in full,
>>> and the score changes fastest around sideways.
>>
>>> ### 1.2 `(… + 1) / 2`
>>>
>>> Moves the cosine's range, −1 to 1, to 0 to 1: add 1 (0 to 2), halve
>>> (0 to 1). Hanging, `(−1 + 1) / 2 = 0`; sideways, `(0 + 1) / 2 = 0.5`;
>>> up, `(1 + 1) / 2 = 1`.
>>>
>>> The code takes the `.mean()` over the poles, so with two or three
>>> poles it is the average of their scores; with one pole it is that
>>> pole's.
>
>> ### 2 `small_control`
>> ```text
>> small_control = (4 + tolerance(u, margin=1, value_at_margin=0, sigmoid='quadratic')) / 5
>>               = (4 + (1 − u²)) / 5
>> ```
>>
>> The "push gently" score. 1 with no push, down to 0.8 at full push
>> either way, never lower. It is the only score that looks at what the
>> policy did rather than where the pole and cart are.
>>
>> ```text
>> push u    force     1 − u²    small_control
>> 0         0 N       1         1
>> ±0.25     ±2.5 N    0.9375    0.9875
>> ±0.5      ±5 N      0.75      0.95
>> ±0.75     ±7.5 N    0.4375    0.8875
>> ±1        ±10 N     0         0.8
>> ```
>>
>> It matters because a policy that is paid for the pole alone can push
>> as hard as it likes, every step, and a policy that chatters between
>> full push each way holds a pole just as well as one that nudges. This
>> score pays the nudger a little more, up to a fifth. It is a nudge, not
>> a rule: a full push still keeps four-fifths of the score.
>>
>>> ### 2.1 `tolerance()`
>>>
>>> `tolerance()` is dm_control's one scoring tool, used by every score
>>> but `upright`[^2]. It answers "how close is this number to where I
>>> want it?" with a number between 0 and 1:
>>>
>>> - **Inside the wanted band, 1.** The band is `bounds`, a lower and
>>>   upper limit. Here, and in sections 3 and 4, `bounds` is left at its
>>>   default `(0, 0)`: the band is the single number 0, and only an
>>>   exact 0 is inside it.
>>> - **Outside the band, less than 1**, falling with the distance to the
>>>   band's nearest edge. The fall is measured in `margin`s: the
>>>   distance is divided by `margin`, so a number one `margin` from the
>>>   band is at distance 1, two `margin`s away at distance 2.
>>> - **At exactly one `margin` away, `value_at_margin`.** The shape of
>>>   the fall, the `sigmoid`, is stretched so that it passes through
>>>   that value at distance 1. Default 0.1.
>>>
>>> ```text
>>> distance = (how far outside the band) ÷ margin
>>> tolerance = 1                   inside the band
>>>           = shape(distance)     outside it
>>> ```
>>>
>>> Each `shape` gives 1 at distance 0 and `value_at_margin` at distance
>>> 1; they differ in how they get there and what they do beyond. The
>>> two the cart-pole uses are the quadratic (2.1.1) and the Gaussian
>>> (3.1).
>>>
>>>> ### 2.1.1 The quadratic shape
>>>> ```text
>>>> shape(distance) = 1 − (distance × scale)²    when distance × scale < 1
>>>>                 = 0                          beyond
>>>> scale = √(1 − value_at_margin)
>>>> ```
>>>>
>>>> An upside-down parabola: 1 at the band, bending down ever faster, and
>>>> cut off at 0 once it gets there. With `value_at_margin` = 0, as here,
>>>> `scale` is `√1 = 1`, so the shape is `1 − distance²`, reaching 0 at
>>>> exactly one `margin`.
>>>>
>>>> It is the one shape in `tolerance()` allowed a `value_at_margin` of
>>>> 0 (with the linear and cosine shapes); the others must stay above 0.
>>>>
>>>> Here, with `margin` = 1 and the band at 0, the distance is `|u| ÷ 1`
>>>> = `|u|`, and the shape is `1 − u²`: 1 with no push, 0.75 at half
>>>> push, 0 at full push. The push can never go past ±1 (the motor's
>>>> `ctrlrange`[^3]), so the cut-off beyond is never reached.
>>>>
>>>> The code's `[0]` takes the first motor's score; the cart-pole has
>>>> one motor.
>>
>>> ### 2.2 `(4 + …) / 5`
>>>
>>> Squeezes the shape's range, 1 down to 0, into 1 down to 0.8: add 4
>>> (5 down to 4), divide by 5 (1 down to 0.8). It is the same as
>>> `0.8 + 0.2 × shape`, the `floor + (1 − floor) × …` of this rig's
>>> scores with a floor of 0.8 (`reward-plan.md`, 3.2). At half push,
>>> where `1 − u²` is 0.75:
>>>
>>> ```text
>>> 1. add 4:        4 + 0.75  = 4.75
>>> 2. divide by 5:  4.75 ÷ 5  = 0.95
>>> ```
>>>
>>> The floor is the highest of the three, 0.8: pushing hard is the
>>> cheapest of the things the reward minds, costing at most a fifth.
>
>> ### 3 `small_velocity`
>> ```text
>> small_velocity = (1 + tolerance(ω, margin=5)) / 2
>>                = (1 + exp(−0.0921 × ω²)) / 2
>> ```
>>
>> The "turn slowly" score. 1 with the pole still, falling as it turns
>> faster, towards 0.5, never lower. It watches the speed everywhere, at
>> the top and the bottom alike, with no zone.
>>
>> ```text
>> ω           exp(−0.0921 ω²)   small_velocity
>> 0 rad/s     1                 1
>> 0.5 rad/s   0.977             0.989
>> 1 rad/s     0.912             0.956
>> 2 rad/s     0.692             0.846
>> 3 rad/s     0.437             0.718
>> 5 rad/s     0.1               0.55
>> 7 rad/s     0.011             0.505
>> 10 rad/s    0.0001            0.500
>> 15 rad/s    0.000             0.500
>> ```
>>
>> It matters because `upright` alone pays a pole whirling round, each
>> time it passes the top, as much as a pole held there for that step.
>> This score halves the pay of a fast pass. Because it has no zone, it
>> also charges a swing-up at the bottom, where it must be fast: a pole
>> passing the bottom at 7 rad/s keeps only 0.505 of its score there.
>> dm_control accepts that, because `upright` is nearly 0 at the bottom
>> anyway, so there is little score to halve. This rig's reward scores
>> the speed differently, only near the top and only above a limit
>> (`reward-plan.md`, section 2, *Why only near the top*).
>>
>>> ### 3.1 `tolerance(ω, margin=5)`: the Gaussian shape
>>> ```text
>>> shape(distance) = exp(−½ × (distance × scale)²)
>>> scale = √(−2 × ln(value_at_margin)) = √(−2 × ln 0.1) = 2.146
>>> ```
>>>
>>> `tolerance()` as in 2.1, with its defaults: the band is 0, and the
>>> shape is the Gaussian, the bell curve[^9]. 1 at the band, flat at
>>> first, then falling faster, then flattening again as it creeps
>>> towards 0, which it never reaches. `exp()` is e (2.718…) raised to
>>> a power[^8]: fed 0 it gives 1, and fed a negative number it shrinks,
>>> never to 0. The minus and the square keep what it is fed at or below
>>> 0, so it only shrinks.
>>>
>>> `scale` is chosen so that the shape passes through `value_at_margin`
>>> at distance 1, whatever `value_at_margin` is. With the default 0.1:
>>>
>>> ```text
>>> 1. ln 0.1          = −2.303       (the power e must be raised to for 0.1)
>>> 2. × −2            =  4.605
>>> 3. √               =  2.146       = scale
>>> 4. at distance 1:  exp(−½ × 2.146²) = exp(−2.303) = 0.1   ✓
>>> ```
>>>
>>> Here the distance is `|ω| ÷ 5`, so the shape is
>>> `exp(−½ × (ω × 2.146 ÷ 5)²)` = `exp(−0.0921 × ω²)`. Turning either
>>> way scores the same, because the square drops the sign.
>>>
>>>> ### 3.1.1 `ω`
>>>>
>>>> The pole's turning speed, in rad/s, straight from MuJoCo's `qvel`
>>>> for the hinge[^10]. The code takes `qvel[1:]`, every joint's speed
>>>> but the slider's, which is the first: with one pole, the hinge's
>>>> alone. The `.min()` takes the lowest score over the poles, so with
>>>> several it is the fastest pole that counts.
>>
>>>> ### 3.1.2 `margin` = 5 rad/s
>>>>
>>>> The speed at which the shape has fallen to `value_at_margin`, 0.1,
>>>> and `small_velocity` to 0.55. A fixed number in the code, not a
>>>> setting[^1]. To picture it: 5 rad/s is the pole's 1 m tip moving at
>>>> about 5 m/s, and about 0.8 of a turn a second.
>>>>
>>>> - **Smaller** (say 2 rad/s): the score falls sooner, so balancing's
>>>>   small corrections start to cost, and a swing-up is charged nearly
>>>>   the full half everywhere it is fast.
>>>> - **Larger** (say 10 rad/s): only a fast whirl is charged; a swing-up
>>>>   passing the bottom at 7 rad/s keeps 0.66 instead of 0.505.
>>
>>>> ### 3.1.3 `value_at_margin` = 0.1
>>>>
>>>> `tolerance()`'s default: at one `margin` away the shape is a tenth.
>>>> Not passed here, so the default holds[^2]. It sets how sharp the
>>>> fall is for a given `margin`: a higher value (say 0.5) gives a
>>>> wider, gentler bell for the same `margin`.
>>
>>> ### 3.2 `(1 + …) / 2`
>>>
>>> Squeezes the shape's range, 1 down towards 0, into 1 down towards
>>> 0.5: add 1 (2 down towards 1), halve (1 down towards 0.5). It is
>>> `0.5 + 0.5 × shape`, a floor of 0.5. At 3 rad/s, where the shape is
>>> 0.437:
>>>
>>> ```text
>>> 1. add 1:   1 + 0.437  = 1.437
>>> 2. halve:   1.437 ÷ 2  = 0.718
>>> ```
>>>
>>> However fast the pole turns, a step keeps at least half of what the
>>> other scores earn. Like this rig's floors (`reward-plan.md`, 3.2, *Why
>>> squeeze instead of cutting off*), the squeeze keeps the shape falling
>>> all the way out, so from any speed there is a little more to gain by
>>> slowing down; but past about 7 rad/s the gain left is about 0.005 of
>>> the score, so in practice the charge is flat there.
>
>> ### 4 `centered`
>> ```text
>> centered = (1 + tolerance(x, margin=2)) / 2
>>          = (1 + exp(−0.5756 × x²)) / 2
>> ```
>>
>> The "keep the cart near the middle" score. 1 with the cart in the
>> middle of the rail, falling as it moves out either way, towards 0.5,
>> never lower.
>>
>> ```text
>> cart from the middle   exp(−0.5756 x²)   centered
>> 0                      1                 1
>> 0.25 m                 0.965             0.982
>> 0.5 m                  0.866             0.933
>> 1 m                    0.562             0.781
>> 1.5 m                  0.274             0.637
>> 1.8 m (the rail's end) 0.155             0.577
>> 2 m (one margin)       0.1               0.55
>> ```
>>
>> It matters for the reason this rig's `r_cart` exists (`reward-plan.md`,
>> section 3): `upright` pays a standing pole the same anywhere on the
>> rail, and a swing-up runs the cart out, so without this the cart may
>> end up anywhere. Here nothing bad happens at the rail's end: the
>> episode does not stop there (section 6), the cart just cannot go
>> further. The score is the only thing that prefers the middle, and it
>> is a nudge: at the very end of the rail a step still keeps 0.577 of its
>> score.
>>
>>> ### 4.1 `tolerance(x, margin=2)`
>>>
>>> The same Gaussian shape as 3.1, with the band at 0 and the default
>>> `value_at_margin` of 0.1, so `scale` is again 2.146. The distance is
>>> `|x| ÷ 2`, and the shape is `exp(−½ × (x × 2.146 ÷ 2)²)` =
>>> `exp(−0.5756 × x²)`. Left and right score the same.
>>>
>>>> ### 4.1.1 `x`
>>>>
>>>> The cart's position along the rail, in metres from the middle,
>>>> straight from MuJoCo's `qpos` for the `slider` joint[^1]. The rail
>>>> allows −1.8 m to 1.8 m[^3].
>>
>>>> ### 4.1.2 `margin` = 2 m
>>>>
>>>> The distance at which the shape has fallen to a tenth and `centered`
>>>> to 0.55. A fixed number in the code[^1]. It is further than the cart
>>>> can go: the rail ends at 1.8 m, where the shape is still 0.155. So
>>>> the score keeps falling over the whole rail and never flattens, and
>>>> the bell's gentle start means the first 0.25 m from the middle costs
>>>> under 0.02. On this rig's rail, 0.44 m to each end, a margin of 2 m
>>>> would make the score nearly flat: 0.947 at an end.
>>
>>> ### 4.2 `(1 + …) / 2`
>>>
>>> The same squeeze as 3.2, a floor of 0.5: wherever the cart is, a step
>>> keeps at least half of what the other scores earn. With the cart 1 m
>>> out, where the shape is 0.562:
>>>
>>> ```text
>>> 1. add 1:   1 + 0.562  = 1.562
>>> 2. halve:   1.562 ÷ 2  = 0.781
>>> ```
>
>> ### 5 The sparse variants
>> ```text
>> r = cart_in_bounds × angle_in_bounds
>> cart_in_bounds  = tolerance(x,     bounds=(−0.25, 0.25))    margin 0
>> angle_in_bounds = tolerance(cos θ, bounds=(0.995, 1))       margin 0
>> ```
>>
>> `balance_sparse` and `swingup_sparse` use a different reward from the
>> same code, with `sparse=True`[^1]: 1 when the cart is within 0.25 m of
>> the middle and the pole within 5.73° of straight up, and 0 otherwise.
>> Nothing in between.
>>
>> With `margin` left at 0, `tolerance()` is 1 inside the band and 0
>> outside, with no fall-off at all (2.1). The band for the angle is on
>> the cosine, 0.995 to 1, and `cos θ` = 0.995 at θ = 5.73°
>> (`arccos 0.995`), so the pole must be within 5.73° either way. The
>> `.prod()` multiplies over the poles, so every pole must be within it.
>>
>> "Sparse" means the policy is paid only on success and told nothing on
>> the way: from hanging, every step scores 0 until the pole is both up
>> and over the middle, and the policy must find that by chance. It is
>> the harder version of each task, kept in the suite to test learning
>> from rare rewards[^5]. The four scores of section 0 are the "smooth"
>> reward, where every step says how close it was.
>
>> ### 6 How an episode is scored
>>
>> An episode is 10 s: `time_limit` is 10, a physics step is 0.01 s, and
>> each policy step is one physics step, so an episode is 1,000 steps[^6],
>> each paying at most 1. A perfect episode scores 1,000; the Control
>> Suite reports its benchmarks over 1,000-step episodes[^6].
>>
>> Nothing ends an episode early. The cart-pole task sets no end
>> condition, so the only end is the time limit[^6]; the cart stopping at
>> the rail's end, or the pole falling, costs nothing on that step beyond
>> what the four scores take. Collisions are switched off in the
>> model[^3], so nothing can hit anything.
>>
>> Where an episode starts[^1], with a little randomness each time so no
>> two are the same:
>>
>> - **`swingup`, `swingup_sparse`:** cart within about 0.01 m of the
>>   middle, pole hanging within about 0.01 rad (0.6°) of straight down,
>>   everything nearly still.
>> - **`balance`, `balance_sparse`:** cart anywhere within 0.1 m of the
>>   middle, pole anywhere within 0.034 rad (1.9°) of straight up,
>>   everything nearly still.
>>
>> The reward is the same in both; only the start differs. In `balance`
>> the first step already scores nearly 1 and the policy's job is to keep
>> it; in `swingup` the first step scores 0 (`upright` is 0 hanging), and
>> the policy's first job is to make `upright` anything but 0.

> ## How it differs from this rig's reward
>
> `trainer/reward-plan.md` is this rig's reward. Where the two part:
>
> - **What the policy sees.** dm_control's policy sees the simulator's
>   true state: the cart's position, the pole's angle and every speed.
>   This rig's policy sees only what the camera's detector finds, and
>   the reward alone sees MuJoCo's numbers (`AGENTS.md`, *Not a toy*).
> - **The pole's score.** dm_control scores the angle's cosine, with no
>   width to tune and no floor: hanging is exactly 0, and the whole
>   reward with it. This rig scores the tip's distance from a star with
>   `exp(−d / w)`, never 0, so a hanging pole still earns 0.0075 and a
>   product with a 0 in it never happens.
> - **The cart's score.** The same idea, a Gaussian in dm_control and an
>   `exp()` here, but dm_control has no free zone and its margin, 2 m,
>   is longer than its rail; this rig's zone is 0.05 m and its width
>   0.25 m on a 0.44 m half-rail. The floors are 0.5 and 0.667.
> - **The speed.** dm_control charges the turning speed everywhere, with
>   a floor of 0.5 and no limit below which it is free. This rig charges
>   it in two ways: only near the top above 0.5 rad/s (`r_target_speed`),
>   and everywhere above 12 rad/s (`r_speed`), each with its own floor.
> - **The push.** dm_control charges a hard push, up to a fifth. This rig
>   does not score the command at all; the rig's motor has its own ramp
>   limit in the firmware instead (`AGENTS.md`, *The Uno is dumb*).
> - **A second star.** This rig's `r_reach` prefers the pole standing up
>   over the middle of the rail from the tip's side as well as the
>   cart's. dm_control has only `centered`.
> - **The episode.** dm_control's is a fixed 10 s with nothing ending it
>   early, and the cart simply stops at the rail's end. This rig's ends
>   when the cart reaches a strike face (`reward-plan.md`, the strike-face
>   sites).
> - **Where the numbers live.** dm_control's margins and floors are fixed
>   in the code. This rig's are each in `trainer/gym.toml`, one table per
>   factor (`AGENTS.md`, *Tunable values live in a toml*).

## References

Where each fact comes from, checked against the submodule's files on
2026-10-31. Paths are inside `submodules/dm_control`, at commit
`e94f1e9` (dm_control's `main` on that day).

[^1]: **The reward, the state it reads, the start states and the
    sparse variants.** `Balance._get_reward`, `Balance.initialize_episode`,
    `Balance.get_observation` and the `Physics` readers
    `cart_position()`, `angular_vel()` and `pole_angle_cosine()`, all in
    [`dm_control/suite/cartpole.py`](../submodules/dm_control/dm_control/suite/cartpole.py).
    The smooth reward is
    `upright.mean() * small_control * small_velocity * centered`; the
    sparse one `cart_in_bounds * angle_in_bounds`. The margins (1, 5, 2)
    and the squeezes (`(4 + …) / 5`, `(1 + …) / 2`) are literals in that
    function. The `balance` tasks start with `qpos['slider']` uniform in
    ±0.1 and the hinge uniform in ±0.034; the `swingup` tasks with
    `0.01 × randn` on the slider and `π + 0.01 × randn` on the hinge;
    both with `0.01 × randn` on every speed (`randn` is a draw from the
    bell curve with spread 1, so "within about 0.01" means one spread).

[^2]: **`tolerance()` and its shapes.** `tolerance(x, bounds=(0.0, 0.0),
    margin=0.0, sigmoid='gaussian', value_at_margin=0.1)` and `_sigmoids`
    in
    [`dm_control/utils/rewards.py`](../submodules/dm_control/dm_control/utils/rewards.py).
    Inside the bounds the value is 1; with `margin` 0 it is 0 outside;
    otherwise `d = (distance to the nearest bound) / margin` is fed to
    the shape. Gaussian: `scale = sqrt(-2 log(value_at_1))`,
    `exp(-0.5 (x scale)²)`. Quadratic: `scale = sqrt(1 - value_at_1)`,
    `1 - (x scale)²` where `|x scale| < 1`, else 0. The quadratic, linear
    and cosine shapes allow `value_at_1` = 0; the rest require
    `0 < value_at_1 < 1`.

[^3]: **The model.**
    [`dm_control/suite/cartpole.xml`](../submodules/dm_control/dm_control/suite/cartpole.xml):
    `timestep="0.01"`, `integrator="RK4"`, `contact="disable"`; the cart
    a box of half-sizes 0.2 × 0.15 × 0.1 m, `mass="1"`, at `pos="0 0 1"`;
    the `slider` joint along `x` with `range="-1.8 1.8"`, limited; the
    pole a capsule `fromto="0 0 0 0 0 1"` (1 m, pointing up at angle 0)
    of radius 0.045 m and `mass=".1"`, on a hinge about `y`; the motor
    `gear="10"`, `ctrlrange="-1 1"`. The rails are drawn 4 m long
    (`size="0.02 2"`, a half-length of 2 m) but the joint's range is what
    stops the cart.

[^4]: **Rewards in the unit interval, through `tolerance()`.** "The
    range of rewards in the Control Suite, with the exception of the
    LQR domain, are in the unit interval", and the structure "is
    facilitated by the tolerance() function"
    ([Tassa et al. 2018, section 2, *Reinforcement Learning*, Reward](https://ar5iv.labs.arxiv.org/html/1801.00690)).
    Read 2026-10-31.

[^5]: **The four cart-pole tasks.** "Swing up and balance an unactuated
    pole by applying forces to a cart at its base", with `balance`,
    `balance_sparse`, `swingup` and `swingup_sparse`; the pole starts
    pointing down in the swing-up tasks and near upright in the balance
    tasks
    ([Tassa et al. 2018, section 3, *Domains and Tasks*, Cart-pole](https://ar5iv.labs.arxiv.org/html/1801.00690)).
    The code tags all four `benchmarking` (`SUITE.add('benchmarking')`
    in `cartpole.py`). Read 2026-10-31.

[^6]: **1,000 steps, no early end.** `_DEFAULT_TIME_LIMIT = 10` in
    `cartpole.py`; `control.Environment` takes `n_sub_steps = 1` when
    neither it nor `control_timestep` is given, and
    `_step_limit = time_limit / (physics.timestep() * n_sub_steps)`
    = 10 / (0.01 × 1) = 1,000
    ([`dm_control/rl/control.py`](../submodules/dm_control/dm_control/rl/control.py)).
    Each step ends the episode only when `_step_count >= _step_limit` or
    when the task's `get_termination` returns a value; the cart-pole
    task does not override it, and the base returns none. The paper:
    "fixed-length episodes of 1000 time steps", and Control Suite tasks
    "have no terminal states or time limit" in principle, the 1,000
    steps being the benchmark's cut
    ([Tassa et al. 2018, section 2, *Reinforcement Learning*](https://ar5iv.labs.arxiv.org/html/1801.00690)).

[^7]: **`xmat`'s `zz` entry is the cosine of the pole's angle.** MuJoCo
    keeps each body's orientation as a 3 × 3 rotation matrix, `xmat`
    ("Cartesian orientation of body frame",
    `include/mujoco/mjdata.h`). Its `zz` entry is the dot product of the
    body's own `z` axis with the world's `z` axis: the cosine of the
    angle between them. The pole's geom runs along its body's `z`
    (`fromto="0 0 0 0 0 1"`, note 3), so that angle is the pole's lean
    from straight up. `pole_angle_cosine()` reads
    `named.data.xmat[2:, 'zz']`: body 0 is the world, body 1 the cart,
    body 2 onward the poles.

[^8]: **e and `exp()`.** e is "the base of the natural logarithm and
    exponential function"
    ([Wikipedia, *e (mathematical constant)*](https://en.wikipedia.org/wiki/E_(mathematical_constant))).
    `ln` is the natural logarithm, the power e must be raised to for a
    number: `ln 0.1 = −2.303` because `exp(−2.303) = 0.1`. dm_control
    uses NumPy's `np.exp` and `np.log`.

[^9]: **The Gaussian, the bell curve.** `exp(−x² / 2)` is the Gaussian
    function, the shape of the normal distribution's bell
    ([Wikipedia, *Gaussian function*](https://en.wikipedia.org/wiki/Gaussian_function)).
    The constants in sections 3 and 4 are worked out from note 2's
    formulas: `scale` = √(−2 ln 0.1) = 2.14597; for `margin` 5,
    ½ × (2.14597 ÷ 5)² = 0.09210; for `margin` 2, ½ × (2.14597 ÷ 2)² =
    0.57565. Every value in this document's tables was computed from
    these formulas on 2026-10-31, not measured in a run.

[^10]: **The speed and the push, straight from MuJoCo.** `physics.control()`
    returns a copy of `data.ctrl` and `physics.velocity()` a copy of
    `data.qvel`
    ([`dm_control/mujoco/engine.py`](../submodules/dm_control/dm_control/mujoco/engine.py));
    `angular_vel()` takes `data.qvel[1:]` and `cart_position()`
    `named.data.qpos['slider'][0]` (`cartpole.py`). `ctrl` is the
    control the policy set for this step, and `qvel` the joint
    velocities after the step.
