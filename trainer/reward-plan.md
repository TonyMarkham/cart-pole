# The reward, explained simply

> ## Where the reward's numbers come from
>
> The robot learns in a simulation, not on the real rig. MuJoCo is the
> physics simulator: it plays the part of the real cart, pole and rail,
> built from the rig's CAD model.
>
> **Every position the reward uses comes from MuJoCo.** MuJoCo knows
> exactly where every part is. The rig's model carries named points,
> called sites[^1], on the parts that matter: the End marker on the pole,
> the Pivot marker on the hinge, the cart itself, and target points the
> pole's tip and the cart are scored against. Each site is a 3D point,
> three numbers in metres:
>
> - `x`: how far along the rail.
> - `y`: how far front to back. Each site's `y` never changes, because
>   the cart only moves along the rail and the pole swings in a flat
>   plane. Pivot, End, Target and Dynamic Target all sit at −0.132;
>   Cart and Cart Target at 0.
> - `z`: how high.
>
> Each position is read at the moment the camera takes its picture, so
> it matches what that picture shows. These are the ground truth: exact,
> with no measurement error.
>
> **MuJoCo also gives the pole's own numbers.** At each capture, with
> the sites: how fast the pole is turning, where its centre of mass is,
> and where the hinge is. Once per run: the pole's mass, how far its centre of mass is from
> the hinge, how hard it is to start turning, and gravity. They are
> ground truth in the same way. The reward reads only the speed (2.1.1.1);
> the once-per-run numbers explain where `target_limit` comes from (2.1.1.3).
>
> **The reward never uses what the camera's detector finds.** The robot
> itself sees the rig only the way the real robot will: through the
> camera, where an object detector finds the four ArUco markers[^3] in
> each picture. Those detections, and the robot's own last commands, are
> all the robot gets to act on. The reward is worked out separately, from
> MuJoCo's exact numbers, and the robot never sees them[^2]. This works
> because training only ever happens in the simulation. The real robot
> never needs to score itself.
>
> The model has twelve sites, below. All twelve reach the trainer; the
> reward uses six: End, Dynamic Target, Target, Pivot, Cart and Cart Target.
> Positions are as at the start of every episode: cart centred, pole
> hanging.
>
>> ### `05-Left.Site` (Left marker)
>>
>> The centre of the ArUco marker on the left end mount. Fixed: it never
>> moves. At (−0.510, −0.077, 0.025). Not used by the reward or the
>> episode end.
>
>> ### `06-Right.Site` (Right marker)
>>
>> The centre of the ArUco marker on the right end mount. Fixed. At
>> (0.495, −0.077, 0.025). Not used by the reward or the episode end.
>
>> ### `07-Pole.Pivot.Site` (Pivot marker)
>>
>> The centre of the marker on the pole's hinge, on the cart. It moves
>> with the cart, along the rail only. At (−0.010, −0.132, 0.082). Used
>> by the reward for its height alone: the tip must be above it for
>> `r_target_speed` to count (2.1.2.3). Where the cart is comes from Cart
>> (3.1.1.1).
>
>> ### `08-Pole.End.Site` (End marker)
>>
>> The centre of the marker near the end of the pole, 0.45 m from the
>> hinge. It moves with the cart and swings with the pole. At
>> (−0.010, −0.132, −0.368) hanging. Used by the reward: its distance
>> from Dynamic Target is `d` (1.1.1), along the rail alone `d_x`
>> (2.1.2.1), and from Target `d_r` (4.1.1).
>
>> ### `09-Pole.End.Target.Site` (Target)
>>
>> The fixed star: where End sits with the cart centred and the pole
>> straight up. Fixed: it never moves. At (−0.010, −0.132, 0.532),
>> 0.45 m above the hinge. Used by the reward: End's distance from it is
>> `d_r` (4.1.1).
>
>> ### `10-Left-Rail.Site` (Left Rail strike face)
>>
>> Where the cart would hit the left base. Fixed. At
>> (−0.485, −0.045, 0.050). Used by the episode end, not the reward.
>
>> ### `11-Left-Cart.Site` (Left Cart strike face)
>>
>> The cart's left face, the part that would hit the left base. It moves
>> with the cart. At (−0.045, −0.045, 0.050), so the cart has 0.440 m to
>> the left base. Used by the episode end.
>
>> ### `12-Right-Rail.Site` (Right Rail strike face)
>>
>> Where the cart would hit the right base. Fixed. At
>> (0.470, −0.045, 0.050). Used by the episode end.
>
>> ### `13-Right-Cart.Site` (Right Cart strike face)
>>
>> The cart's right face. It moves with the cart. At
>> (0.025, −0.045, 0.050), so the cart has 0.445 m to the right base.
>> Used by the episode end.
>
>> ### `14-Dynamic.End.Target.Site` (Dynamic Target)
>>
>> The moving star: where End sits with the pole straight up, wherever
>> the cart is. It is fixed to the cart, 0.45 m straight above the
>> hinge, so it slides with the cart and never tilts. At
>> (−0.010, −0.132, 0.532) with the cart centred, the same point as
>> Target; with the cart 0.2 m left, at (−0.210, −0.132, 0.532). Used by
>> the reward: End's distance from it is `d` (1.1.1), and along the rail
>> alone `d_x` (2.1.2.1).
>
>> ### `15-Cart.Target.Site` (Cart Target)
>>
>> The middle of the rail: where Cart sits with the cart centred, which
>> is where every episode starts. Fixed. At (−0.010, 0.000, 0.050).
>> Used by the reward: Cart's distance from it is how far the cart is
>> from the middle, in `c_d` (3.1.1).
>
>> ### `16-Cart.Site` (Cart)
>>
>> The cart itself: the point where the cart rides on the rail's slider
>> joint. It moves with the cart, along the rail only. At
>> (−0.010, 0.000, 0.050). Used by the reward, in `c_d` (3.1.1).

> ## 0 The Reward
> ```text
> r = r_tip × r_target_speed × r_cart × r_reach × r_speed
> ```
> 
> After every step the robot gets a score, `r`, between 0 and 1. Its
> whole job is to collect as much score as it can, step after step. The
> robot never sees how the score is worked out; it only gets the number.
> 
> The score is five smaller scores multiplied[^4]: one for getting the
> pole's tip straight above the hinge (`r_tip`), one for not whipping
> past the top too fast (`r_target_speed`), one
> for keeping the cart near the middle of the rail (`r_cart`), one
> for the tip being above the middle of the rail (`r_reach`), and one for
> never turning much too fast anywhere (`r_speed`). Multiplying
> means all five matter: a bad mark on any of them drags the whole score
> down, however good the others are. With the cart in the middle: pole
> straight up, not moving: 1. Pole whirling round, passing straight up
> at 10 rad/s: 0.667. Pole sticking out sideways, still: 0.028. Pole
> hanging still: 0.0075. Pole straight up and still, but the cart 0.2 m
> from the middle: 0.67 (the cart is outside its free zone, section 3,
> and the tip is 0.2 m from the point above the middle, section 4).
> 
>> ### Reasoning
>>
>> The score always obeys `0 < r ≤ 1`[^5]: more than 0, and at most 1.
>> Never 0, because in a product a single 0 wipes out every other
>> score: a pole perfectly still but hanging, or perfectly on the star
>> but whirling, would score the same 0 as the worst step possible, and
>> the robot could not tell what it had done well. So every step pays
>> something, and there is always something to gain by doing better.
>> At most 1, so a perfect step is the ceiling
>> and nothing can pay more.
>> 
>> For that to hold, every score multiplied into `r` must obey the same
>> rule: `0 < r_tip ≤ 1`, `0 < r_target_speed ≤ 1`, `0 < r_cart ≤ 1`,
>> `0 < r_reach ≤ 1` and `0 < r_speed ≤ 1`.
>> Multiplying numbers that
>> are each more than 0 and at most 1 always gives a number that is more
>> than 0 and at most 1. Break the rule once and `r` breaks with it: a
>> score above 1 could push `r` past 1, a score of 0 would wipe out the
>> other scores and make `r` 0, and a negative score would make `r`
>> negative. Each score is built on an `exp()` of something never above
>> 0, and `exp()` never gives 0: `r_tip` gives 1 only on the star
>> (1.1.4), `r_target_speed` gives 1 unless the pole is over its speed limit
>> near the top, and never less than its floor, 0.667 (2), `r_cart` gives 1 only with the cart in
>> its free zone, and never less than its floor, 0.667 (3),
>> `r_reach` gives 1 only with the tip above the middle of the rail, and
>> never less than its floor, 0.667 (4), and `r_speed` gives 1 at or below
>> its speed limit, and never less than its floor, 0.250 (5).
>> 
>> **Why no score goes above 1.** In a product, a score above 1 can
>> cancel another score's cut. If `r_tip` could pay 10 while `r_target_speed`
>> cut to 0.1 for a whirling pole, `r` would be 1, as if the step were
>> perfect. The score above 1 skews `r`, and the cut it cancels loses its
>> effect. With every score at most 1, every cut always counts.
>> 
>> The reward's size also changes how the robot learns. DQN held every
>> reward within a fixed range, −1 to +1, because that "limits the scale
>> of the error derivatives" and let one learning rate work across all
>> its games
>> ([Mnih et al. 2013, section 5](https://ar5iv.labs.arxiv.org/html/1312.5602)).
>> SAC's authors found that the reward's size sets the balance between
>> trying new things and repeating what pays: too small and the robot
>> acts almost at random, too large and it stops exploring and gets stuck
>> ([Haarnoja et al. 2018, section 5.2](https://ar5iv.labs.arxiv.org/html/1801.01290)).
>> Their later version, the one this trainer uses (Stable-Baselines3's
>> automatic entropy coefficient), adjusts for the reward's size as it
>> learns, because the right setting shifts as the reward changes during
>> training
>> ([Haarnoja et al. 2018b, section 5](https://ar5iv.labs.arxiv.org/html/1812.05905)).
>> Keeping `r` between 0 and 1 holds that size fixed while the reward's
>> design changes.
>>
> 
>
>> ### 1 `r_tip`
>> ```text
>> r_tip   = exp(-d / w)
>> ```
>>
>> The "get the tip to the gold star" score[^6]. Imagine a gold star
>> carried by the cart on a stick, straight above the hinge, where the
>> pole's tip would be if the pole stood straight up. The closer the tip
>> is to the star, the higher the score: 1 on the star, smaller the
>> further away, never quite 0. The star moves with the cart, so a pole
>> standing straight up scores 1 anywhere on the rail.
>>
>>> ### Why the star moves with the cart
>>>
>>> Until 2026-10-29 the star was fixed in the air above the middle of
>>> the rail (Target, now `r_reach`'s, section 4). With the cart off
>>> the middle, the tip could not reach that star with the pole straight
>>> up: the nearest it could get was with the pole leaning towards the
>>> middle. So the fixed star paid most for a lean, and a leaning pole
>>> always falls: the score pointed the robot at a pose it can never
>>> hold[^23]. With the pole still, the fixed star's `r_tip` for the best
>>> lean and for straight up:
>>>
>>> ```text
>>> cart from the middle   best lean   it pays   straight up pays
>>> 0.1 m                  12.5°       0.947     0.607
>>> 0.2 m                  24.0°       0.809     0.368
>>> 0.3 m                  33.7°       0.635     0.223
>>> ```
>>>
>>> The moving star pays straight up best wherever the cart is. Keeping
>>> the cart near the middle is left to `r_cart` (section 3), whose job
>>> it is.
>>>
>>> ### 1.1 `exp(-d / w)`
>>> 
>>> A score that shrinks to about a third for every `w` the tip is from
>>> the star. `exp()` is the shrinking machine explained in 1.1.4: feed
>>> it 0 and it gives 1, and each −1 fed to it shrinks the answer to
>>> 0.368 of what it was (1 ÷ 2.718, where 2.718 is the number e). One
>>> `w` away, `-d / w` is −1, so the score is `exp(-1)` = 0.368, about a
>>> third. Each further `w` multiplies by that 0.368 again. On the star,
>>> 1. One `w` away, 0.368. Two `w` away, 0.368 × 0.368 = 0.135, about
>>> 1/9. Three `w` away, 0.368 × 0.368 × 0.368 = 0.050, about 1/20.
>>> Every bit closer pays more, so from anywhere there is a direction to
>>> improve.
>>> 
>>>> ### 1.1.1 `d`
>>>> ```text
>>>> d = |End − Dynamic Target|
>>>> ```
>>>>
>>>> How far the pole's tip is from the gold star, in metres, measured in
>>>> a straight line, like a tape stretched from one to the other.
>>>>
>>>>> ### 1.1.1.1 `End`
>>>>>
>>>>> The End marker's MuJoCo site, `08-Pole.End.Site`: the point near
>>>>> the end of the pole, away from the hinge. See its sub-section under
>>>>> *Where the reward's numbers come from*.
>>>>>
>>>>
>>>>> ### 1.1.1.2 `Dynamic Target`
>>>>>
>>>>> The gold star's MuJoCo site, `14-Dynamic.End.Target.Site`: where
>>>>> End sits with the pole straight up, wherever the cart is. See its
>>>>> sub-section under *Where the reward's numbers come from*.
>>>>>
>>>>
>>>>> ### 1.1.1.3 `|End − Dynamic Target|`
>>>>>
>>>>> **Subtracting two 3D points.** `End − Dynamic Target` subtracts them one
>>>>> number at a time: `x` from `x`, `y` from `y`, `z` from `z`. The
>>>>> result is three numbers, an arrow from the star to the tip: how far
>>>>> along the rail, how far front to back, how far up. Call them `Δx`,
>>>>> `Δy` and `Δz` (Δ, "delta", means "the difference in").
>>>>>
>>>>> Pole hanging, cart centred, End at (−0.010, −0.132, −0.368) and
>>>>> Dynamic Target at (−0.010, −0.132, 0.532):
>>>>>
>>>>> ```text
>>>>> Δx = −0.010 − (−0.010) =  0
>>>>> Δy = −0.132 − (−0.132) =  0
>>>>> Δz = −0.368 −   0.532  = −0.900
>>>>> ```
>>>>>
>>>>> **From the arrow to a distance.** The bars `| |` are the absolute
>>>>> value. On a single number, the absolute value drops the sign:
>>>>> |−3| = 3. On an arrow, it gives the arrow's length, also called
>>>>> its magnitude, using Pythagoras' rule in 3D[^7]: square each of the
>>>>> three numbers, add them up, and take the square root.
>>>>>
>>>>> ```text
>>>>> |End − Dynamic Target| = √(Δx² + Δy² + Δz²)
>>>>> ```
>>>>>
>>>>> Hanging, step by step:
>>>>>
>>>>> ```text
>>>>> 1. the differences:  Δx = 0,  Δy = 0,  Δz = −0.900
>>>>> 2. square each:      0² = 0,  0² = 0,  (−0.900)² = 0.810
>>>>> 3. add them:         0 + 0 + 0.810 = 0.810
>>>>> 4. square root:      √0.810 = 0.900 m
>>>>> ```
>>>>>
>>>>> Here the length happens to equal `Δz` with its sign dropped, only
>>>>> because `Δx` and `Δy` are both 0.
>>>>>
>>>>> Squaring makes every part positive, so the length never depends on
>>>>> which way the arrow points, and it's never negative. Both points
>>>>> always have `y` = −0.132, so `Δy` is always 0 and only `x` and `z`
>>>>> count.
>>>>>
>>>>> Pole sticking out sideways to the right, cart centred: End is at
>>>>> (0.440, −0.132, 0.082).
>>>>>
>>>>> ```text
>>>>> Δx =  0.440 − (−0.010) =  0.450
>>>>> Δy = −0.132 − (−0.132) =  0
>>>>> Δz =  0.082 −   0.532  = −0.450
>>>>> ```
>>>>>
>>>>> ```text
>>>>> 1. the differences:  Δx = 0.450,  Δy = 0,  Δz = −0.450
>>>>> 2. square each:      0.450² = 0.2025,  0² = 0,  (−0.450)² = 0.2025
>>>>> 3. add them:         0.2025 + 0 + 0.2025 = 0.405
>>>>> 4. square root:      √0.405 = 0.636 m
>>>>> ```
>>>>>
>>>>> Neither `Δx` nor `Δz` alone gives it.
>>>>>
>>>>> The distance is 0 only when the tip is on the star. Pole leaning
>>>>> 10°: 0.08 m. Pole straight up with the cart 0.2 m off centre: 0,
>>>>> because the star moves with the cart. Hanging down: 0.9 m.
>>>>>
>>>>
>>>
>>>> ### 1.1.2 `w`
>>>>
>>>> `w` is a distance, in metres, that we choose. It decides how close
>>>> the tip has to get to the star before the score gets big. It is set
>>>> as `width` in `trainer/gym.toml`'s `[reward.tip]`; today it is 0.2 m[^8].
>>>>
>>>> It matters because it decides what the robot gets paid for: being
>>>> exactly on the star, or just getting near it.
>>>>
>>>> Changing it changes `r_tip` (section 1):
>>>>
>>>> - **Smaller `w`** (say 0.1 m): the score only gets big very close to
>>>>   the star. The robot is pushed hard to be exactly on it, but a swing
>>>>   that doesn't get near the top earns almost nothing.
>>>> - **Larger `w`** (say 0.4 m): the score is already decent when the
>>>>   tip is roughly near the star. Getting partway up earns more, but
>>>>   there is less push to be exactly on it.
>>>>
>>>
>>>> ### 1.1.3 `-d / w`
>>>>
>>>> `-d / w` is how many `w`s fit between the tip and the star, with a
>>>> minus sign in front. On the star it is 0. One `w` away it is −1.
>>>> Hanging down, with today's `w` of 0.2 m, it is 0.9 ÷ 0.2 = 4.5, so
>>>> −4.5.
>>>>
>>>> It matters because it is what goes into `exp()` (1.1.4):
>>>>
>>>> - **Dividing by `w`** means the score counts the distance in `w`s,
>>>>   not in metres. That is why changing `w` changes the score (1.1.2).
>>>> - **The minus sign** makes `exp()` shrink the score as the tip gets
>>>>   further from the star, instead of growing it.
>>>>
>>> 
>>>> ### 1.1.4 `exp()`
>>>>
>>>> `exp()` is the number e (2.718…)[^9] raised to the power of whatever it
>>>> is fed.
>>>>
>>>> - Fed 0, it gives 1: any number raised to the power 0 is 1.
>>>> - Fed a positive number, it grows: `exp(1)` = 2.718, `exp(2)` =
>>>>   2.718 × 2.718 = 7.39.
>>>> - Fed a negative number, it shrinks, because a negative power means
>>>>   "1 divided by": `exp(-1)` = 1 ÷ 2.718 = 0.368, `exp(-2)` =
>>>>   1 ÷ 7.39 = 0.135. Each further −1 multiplies the answer by 0.368
>>>>   again, so it gets smaller and smaller but never reaches 0.
>>>>
>>>> How it modifies `-d / w`: `d` is never negative and `w` is always
>>>> more than 0, so `-d / w` is never above 0. That keeps `exp()` always
>>>> on its shrinking side. On the star, `-d / w` is 0 and the score is 1.
>>>> One `w` away it is −1 and the score is 0.368. Further away, the score
>>>> is smaller still, never 0 and never above 1. Without the minus,
>>>> `exp()` would be on its growing side, and the score would get bigger
>>>> the further the tip is from the star.
>>>>
>>
>
>> ### 2 `r_target_speed`
>> ```text
>> r_target_speed = target_floor + (1 - target_floor) × exp(-over_target / target_width)
>> ```
>>
>> The "don't whip past the star" score[^25]. It looks at one thing only:
>> the pole turning too fast while the tip is near the top. Then it cuts
>> the step's score, harder the faster the pole is going, towards its floor,
>> 0.667, but never quite reaching it. Everywhere else it is exactly 1, and
>> in a product a 1 changes nothing.
>>
>> - **Pole held straight up:** 1. Its speed is about 0, so this score costs
>>   a balanced pole nothing, wherever the cart is.
>> - **Near the top, at or below `target_limit`:** 1. The small corrections
>>   balancing needs are free here. They are not free overall: `r_tip` and
>>   `r_reach` still charge for the lean itself.
>> - **Tip away from the top, at any speed:** 1. Pumping the swing up at the
>>   bottom costs nothing.
>> - **Near the top, above `target_limit`:** lower, the faster it goes, but a
>>   step always keeps at least two-thirds of its score.
>>
>> It matters because without a cut near the top, spinning the pole round
>> and round pays well[^10]: every turn sweeps the tip past the star
>> (section 1). Each pass through the zone now gets cut. Like `r_cart` and
>> `r_reach`, it is a nudge, not a rule: it can only cost so much.
>>
>> With today's values, `target_limit` = 0.5 rad/s (2.1.1.3), `zone_dx` =
>> 0.154 m (2.1.2.2), `target_width` = 1 rad/s (2.1.3) and `target_floor` =
>> 0.667 (2.2.1):
>>
>> ```text
>> pole                                in the zone?   over_target   r_target_speed
>> straight up, still                  yes            0             1
>> straight up, turning at 0.5 rad/s   yes            0             1
>> passing straight up at 1 rad/s      yes            0.5           0.869
>> passing straight up at 2 rad/s      yes            1.5           0.741
>> passing straight up at 3 rad/s      yes            2.5           0.694
>> passing straight up at 5.25 rad/s   yes            4.75          0.670
>> passing straight up at 10 rad/s     yes            9.5           0.667
>> passing sideways at 7.4 rad/s       no             0             1
>> passing the bottom at 10.5 rad/s    no             0             1
>> hanging, still                      no             0             1
>> ```
>>
>>> ### Why only near the top
>>>
>>> The obvious way to stop the spin is to score the turning speed: the
>>> faster the pole turns, the lower the score. Scored everywhere, it cannot
>>> work, because at the bottom the speed cannot tell a swing-up from a spin.
>>>
>>> - **To reach the top from hanging, the pole must be fast at the
>>>   bottom.** With exactly enough energy to stop at the top, it passes
>>>   the bottom at 10.5 rad/s (2.1.1.3).
>>> - **A spin passes the bottom at least that fast**, because it has
>>>   more energy still.
>>>
>>> So a speed score everywhere charges the swing-up the robot needs as much
>>> as the spin it must not do. (`r_speed`, section 5, does watch the speed
>>> everywhere, but only above any speed a swing-up needs.)
>>>
>>> Near the top the two differ. A swing-up with exactly enough energy has
>>> slowed to 1.82 rad/s by the zone's edge (2.1.1.3). The spins in one
>>> training run averaged 2.9 to 5.9 rad/s on their passes over the top[^26].
>>>
>>
>>> ### What each kind of play earns
>>>
>>> Over a whole 1,500-step episode, with today's values, from play recorded
>>> in a training run and scored with this reward[^26]. It shows what each
>>> kind of play pays. It does not predict what the robot will learn to do.
>>>
>>> The recording is from before the command rate (2026-10-30,
>>> `synthetic-loop-dia.md`), when a step lasted 12.57 ms and an episode
>>> 1,500 steps. An episode is now about the same 19 s in 460 steps, so the
>>> same play now scores about 460 ÷ 1,500, or 0.31, of each figure: a hold
>>> pays 460, hanging about 3.4 (measured: 3.4 and 3.7 over two episodes of
>>> random commands). That scale is worked out, not measured for the spins.
>>> The reward's calculation did not change. The comparisons between the
>>> kinds of play hold either way.
>>>
>>> ```text
>>> play                            episode score
>>> held straight up                1,500  (worked out)
>>> slower spins, five episodes     116 to 213
>>> the ten fastest spins           36 to 49
>>> hanging about                   12
>>> ```
>>>
>>> **So a hold pays the most, and spinning still pays.** A hold earns about
>>> 7 times the best spin. A slower spin earns 9 to 17 times what hanging
>>> about does, and is far easier to find than a hold. `r_speed` (section 5)
>>> takes 60% to 65% off the fastest spins and under 1% off the slower ones,
>>> so the very fast spins earn only 3 to 4 times what hanging about does.
>>>
>>> Two things limit the cut on a slower spin. The floor keeps at least two-thirds of every
>>> pass through the zone, however fast. And 38% to 63% of a spin's score
>>> comes from outside the zone, nearly all of that in the upper half of the
>>> turn, where `r_target_speed` is 1. A pole 30° from straight up is outside the
>>> zone and still has `r_tip` = 0.31. A wider zone charges more of the turn
>>> (2.1.2.2), but with the whole upper half charged these spins still earn
>>> 101 to 190.
>>>
>>
>>> ### What it does not do
>>>
>>> - **Pumping has no score of its own.** Only the tip rising, through
>>>   `r_tip`, pays for it. A pole hanging still gets 0.0075 a step.
>>> - **A slow tumble is free of this score.** A pole going over the top at
>>>   or below `target_limit` is never charged, however many turns it makes.
>>>   It earns less than a hold, because most of each turn is away from the
>>>   star.
>>> - **The score jumps at the zone's edge.** A fast pole goes from 1 just
>>>   outside the zone to its full cut just inside. A slow pole sees no
>>>   change. Whether that jump troubles the learning is not known.
>>>
>>
>>> ### 2.1 `exp(-over_target / target_width)`
>>>
>>> The same shrinking machine as `r_tip`'s (1.1): 1 when `over_target` is 0, 0.368
>>> (about a third) one `target_width` past the limit, and 0.368 times smaller
>>> again for every further `target_width`, never 0. 1.1.4 explains `exp()`.
>>>
>>>> ### 2.1.1 `over_target`
>>>> ```text
>>>> over_target = max(0, |speed| − target_limit)   when the tip is in the zone
>>>> over_target = 0                               when it is not
>>>> ```
>>>>
>>>> How far past the speed limit the pole is going, in rad/s, counted only
>>>> in the zone. 0 is the good case: there is nothing to charge, and
>>>> `r_target_speed` is 1. The bigger `over_target` is, the worse.
>>>>
>>>>> ### 2.1.1.1 `speed`
>>>>>
>>>>> How fast the pole is turning at the moment of the capture, straight
>>>>> from MuJoCo: the hinge's own turning speed, which MuJoCo calls
>>>>> `qvel`[^13]. It is 0 when the pole is still, bigger the faster it
>>>>> turns, and negative when it turns the other way.
>>>>>
>>>>> It is measured in radians per second (rad/s). A radian is a slice
>>>>> of a turn: 6.28 radians make one full turn (2 × π, where π = 3.14).
>>>>> So 6.28 rad/s is one full turn every second.
>>>>>
>>>>> It is read at the capture, from the same simulator step as the
>>>>> sites, so it is the speed at the instant the picture shows. It is
>>>>> exact: MuJoCo works it out as part of the physics, so it is not an
>>>>> average over the time between two readings.
>>>>>
>>>>
>>>>> ### 2.1.1.2 `|speed|`
>>>>>
>>>>> The speed with its sign dropped (1.1.1.3): turning left and turning
>>>>> right count the same.
>>>>>
>>>>
>>>>> ### 2.1.1.3 `target_limit`
>>>>>
>>>>> The speed, in rad/s, up to which the pole is not charged in the zone.
>>>>> We choose it. It is set as `limit` in `trainer/gym.toml`'s
>>>>> `[reward.target_speed]`; its starting value is 0.5 rad/s[^25].
>>>>>
>>>>> To picture it: multiply by the 0.45 m from the hinge to End, and
>>>>> 0.5 rad/s is End moving at about 0.5 × 0.45 = 0.225 m/s, 22 cm every
>>>>> second. That is a rough guide for the eye, not an exact figure: it
>>>>> ignores the cart's own movement, and the score reads the turning speed,
>>>>> not End's speed.
>>>>>
>>>>> The limit also decides how a swing-up is treated on its way in. A pole
>>>>> with exactly enough energy to stop at the top has, at each lean, just
>>>>> the moving energy it needs to climb the height still left[^12]:
>>>>>
>>>>> ```text
>>>>> moving energy = height energy still to climb
>>>>> ½ × (speed / ω0)² = 1 − cos(lean)
>>>>> speed = ω0 × √(2 × (1 − cos(lean)))
>>>>> ```
>>>>>
>>>>> `lean` is the angle from straight up, and `cos(lean)` is how much of
>>>>> the pole's height it still stands at: 1 straight up, 0 sideways, −1
>>>>> hanging. Both sides of the second line count energy in the same unit,
>>>>> the energy it takes to lift the pole's weight from level with the
>>>>> hinge to its height straight up. `ω0` is 2.1.1.4.
>>>>>
>>>>> That formula gives 10.5 rad/s at the bottom (`lean` = 180°) and
>>>>> 1.82 rad/s at 20°. The hinge's friction takes a little energy on the way
>>>>> up[^16], so a real arrival needs a hair more at the bottom.
>>>>>
>>>>> Each limit, the speed of End it means, and the `d_x` (2.1.2.1) beyond
>>>>> which a pole with exactly enough energy is over that limit:
>>>>>
>>>>> ```text
>>>>> target_limit         End speed    over the limit beyond d_x of
>>>>> 2 rad/s             0.9 m/s      0.168 m
>>>>> 1 rad/s             0.45 m/s     0.085 m
>>>>> 0.5 rad/s (today)   0.225 m/s    0.043 m
>>>>> 0.25 rad/s          0.11 m/s     0.021 m
>>>>> ```
>>>>>
>>>>> With today's values, the zone reaches past 0.043 m, so an arrival with
>>>>> exactly enough energy is charged on its way in: 0.756 at the zone's
>>>>> edge, `d_x` = 0.154 m, and 0.888 at `d_x` = 0.078 m, rising to 1 by
>>>>> 0.043 m. So the robot is paid to arrive slower than that, braking with
>>>>> the cart.
>>>>>
>>>>> Changing it changes `r_target_speed` (section 2):
>>>>>
>>>>> - **Lower** (say 0.25 rad/s): less free motion. Corrections have to be
>>>>>   gentler, and a swing-up is charged closer to the top.
>>>>> - **Higher** (say 2 rad/s): more free motion, but a spin passing the top
>>>>>   at 2 rad/s goes uncharged. It sets how slow a spin can be and still
>>>>>   escape this score.
>>>>>
>>>>
>>>>> ### 2.1.1.4 `ω0`
>>>>> ```text
>>>>> ω0 = √(m × g × l_c ÷ I)
>>>>> ```
>>>>>
>>>>> `ω0` ("omega nought") is how fast the pole swings back and forth on
>>>>> its own, in small swings, with the cart standing still: its natural
>>>>> frequency[^14], in rad/s. It is not a setting: it comes from the
>>>>> pole itself, and changes only if the pole is rebuilt. For this pole
>>>>> it is 5.25 rad/s, one small swing there and back every
>>>>> 6.28 ÷ 5.25 = 1.20 s, which can be timed by hand on the real rig.
>>>>> `r_target_speed` itself does not use it: it is here only to show where
>>>>> `target_limit`'s table comes from.
>>>>>
>>>>> Its four numbers come from MuJoCo once per run, read from the rig's
>>>>> model (the MJCF) when the simulator loads it[^15]. "The pole" is
>>>>> everything that swings on the pole's hinge (`04-Pole.Hinge.joint`):
>>>>> the rod, the markers, the holders, the screws.
>>>>>
>>>>> - `m`: their mass, 0.402 kg.
>>>>> - `g`: gravity, 9.81 m/s², MuJoCo's own value; the model does not
>>>>>   change it.
>>>>> - `l_c`: how far their centre of mass (the balance point of the
>>>>>   whole swinging part) is from the hinge, 0.155 m.
>>>>> - `I`: their moment of inertia about the hinge, 0.0222 kg·m²: how
>>>>>   hard the pole is to start turning. Weight far from the hinge
>>>>>   counts more than weight near it.
>>>>>
>>>>> ```text
>>>>> m × g × l_c = 0.402 × 9.81 × 0.155 = 0.613
>>>>> ÷ I:          0.613 ÷ 0.0222       = 27.5
>>>>> √:            √27.5                = 5.25 rad/s
>>>>> ```
>>>>>
>>>>
>>>>> ### 2.1.1.5 `max(0, … − target_limit)`
>>>>>
>>>>> Taking `target_limit` away counts the speed from the limit instead of
>>>>> from still. Below the limit that leaves a number below 0, and
>>>>> `max(0, …)`, "whichever is bigger, 0 or this", makes `over_target` 0.
>>>>>
>>>>> ```text
>>>>> speed 0.3 rad/s:  max(0, 0.3 − 0.5) = max(0, −0.2) = 0
>>>>> speed 3 rad/s:    max(0, 3 − 0.5)   = max(0, 2.5)  = 2.5 rad/s
>>>>> ```
>>>>>
>>>>
>>>
>>>> ### 2.1.2 The zone
>>>> ```text
>>>> in the zone = d_x ≤ zone_dx  and  End.z > Pivot.z
>>>> ```
>>>>
>>>> Where the speed counts: the tip is near the point straight above the
>>>> hinge, along the rail, and it is above the hinge. Both must be true.
>>>>
>>>>> ### 2.1.2.1 `d_x`
>>>>> ```text
>>>>> d_x = |End.x − Dynamic Target.x|
>>>>> ```
>>>>>
>>>>> How far along the rail the tip is from the gold star, in metres: only
>>>>> the `x` part of the arrow in 1.1.1.3, its `Δx`, with the sign dropped.
>>>>> `r_tip` keeps its full `d`. The star rides on the cart, so moving the
>>>>> cart does not change `d_x`: a pole straight up has `d_x` = 0 anywhere
>>>>> on the rail.
>>>>>
>>>>> Pole leaning 20° to the right, cart centred: End is 0.45 m × sin(20°)
>>>>> = 0.154 m to the right of the star. `sin(lean)` is the share of the
>>>>> pole's length that points along the rail: 0 straight up, 1 sideways.
>>>>>
>>>>> ```text
>>>>> End.x             =  0.144
>>>>> Dynamic Target.x  = −0.010
>>>>> d_x = |0.144 − (−0.010)| = 0.154 m
>>>>> ```
>>>>>
>>>>
>>>>> ### 2.1.2.2 `zone_dx`
>>>>>
>>>>> How big the zone is: the largest `d_x` that still counts, a distance
>>>>> in metres that we choose. It is set as `zone_dx` in
>>>>> `trainer/gym.toml`'s `[reward.target_speed]`; its starting value is 0.154 m,
>>>>> a lean of 20°[^25], the band the training measures already use
>>>>> (`trainer/measures.toml`).
>>>>>
>>>>> Changing it changes which part of a turn is charged, with a cost on each
>>>>> side. A wider zone charges more of a spin, but also more of a real
>>>>> swing-up, which is above `target_limit` for any `d_x` past about 0.043 m
>>>>> (the table in 2.1.1.3). The five slower spins of *What each kind of
>>>>> play earns*, with `target_limit` = 0.5 and `target_width` = 1:
>>>>>
>>>>> ```text
>>>>> zone_dx              lean    score of the episodes
>>>>> 0.043 m              5.5°    124.7 to 240.0
>>>>> 0.085 m              11°     120.7 to 227.3
>>>>> 0.154 m (today's)    20°     116.4 to 212.8
>>>>> 0.225 m              30°     112.4 to 202.6
>>>>> 0.318 m              45°     106.8 to 195.4
>>>>> 0.45 m               90°     100.9 to 189.6
>>>>> ```
>>>>>
>>>>> Hanging about earns 12.3 in every row, because its tip never enters
>>>>> the zone.
>>>>>
>>>>
>>>>> ### 2.1.2.3 `End.z > Pivot.z`
>>>>>
>>>>> The tip is above the hinge: the End marker's height (`08-Pole.End.Site`)
>>>>> is above the Pivot marker's (`07-Pole.Pivot.Site`, 0.082 m). It is
>>>>> needed because `d_x` alone cannot tell the top from the bottom. A
>>>>> hanging pole also has its tip straight under the star, so `d_x` is 0
>>>>> there too (1.1.1.3), and without this test the zone would switch on at
>>>>> the bottom, charging the swing-up at 10.5 rad/s, where it most needs its
>>>>> speed.
>>>>>
>>>>
>>>
>>>> ### 2.1.3 `target_width`
>>>>
>>>> A speed, in rad/s, that we choose: how far past the limit the pole can
>>>> go before the part of `r_target_speed` above the floor shrinks to about a
>>>> third. It is set as `width` in `trainer/gym.toml`'s
>>>> `[reward.target_speed]`; its starting value is 1 rad/s[^25].
>>>>
>>>> It decides how quickly a fast pass falls to the floor. `r_target_speed` at each
>>>> speed in the zone, with `target_limit` = 0.5 rad/s and `target_floor` =
>>>> 0.667:
>>>>
>>>> ```text
>>>> target_width   1 rad/s   2 rad/s   3 rad/s   5.25 rad/s
>>>> 0.5 rad/s     0.790     0.684     0.669     0.667
>>>> 1 rad/s       0.869     0.741     0.694     0.670
>>>> 2 rad/s       0.926     0.824     0.762     0.698
>>>> ```
>>>>
>>>> - **Smaller** (say 0.5 rad/s): a sharp cut just past the limit, so a
>>>>   small overshoot on the way to a catch costs nearly all the floor allows.
>>>> - **Larger** (say 2 rad/s): a gentle cut, so a fast pass keeps more of
>>>>   its score.
>>>>
>>>
>>>> ### 2.1.4 `-over_target / target_width`
>>>>
>>>> How many `target_width`s the pole is past the limit, with a minus sign
>>> in front, as in 1.1.3. At or below the limit it is 0. At 3 rad/s in
>>>> the zone it is −(2.5 ÷ 1) = −2.5.
>>>>
>>>
>>
>>> ### 2.2 `target_floor + (1 - target_floor) × …`
>>>
>>> The same squeeze as `r_cart`'s (3.2): `exp()`'s range, from 1 down
>>> towards 0, squeezed into a range from 1 down towards `target_floor`. At
>>> 3 rad/s in the zone, where `exp(-over_target / target_width)` is 0.082:
>>>
>>> ```text
>>> 1. the room above the floor:  1 − 0.667      = 0.333
>>> 2. the share of it kept:      0.333 × 0.082  = 0.027
>>> 3. on top of the floor:       0.667 + 0.027  = 0.694
>>> ```
>>>
>>>> ### 2.2.1 `target_floor`
>>>>
>>>> The lowest `r_target_speed` can be, approached at high speed but never reached.
>>>> It is set as `floor` in `trainer/gym.toml`'s `[reward.target_speed]`; its
>>>> starting value is 0.667[^25]: however fast the pole passes through the
>>>> zone, a step keeps at least two-thirds of the score the other scores earn.
>>>>
>>>> - **Higher** (say 0.9): a gentler nudge. A fast pass can cost at most a
>>>>   tenth of the score.
>>>> - **Lower** (say 0.2): a firmer one, up to four-fifths.
>>>>
>>>
>>
>
>> ### 3 `r_cart`
>> ```text
>> r_cart = cart_floor + (1 - cart_floor) × exp(-c_d / c_w)
>> ```
>>
>> The "keep the cart near the middle" score[^19]. Anywhere within
>> 0.050 m of the middle of the rail, either way, it is 1: the cart's
>> free zone, where it can move about at no cost. Past the zone it
>> shrinks the further the cart goes, towards its floor, 0.667, but never
>> quite reaches it.
>>
>> It matters because the tip score does not keep the cart near the
>> middle. `r_tip` pays a pole standing straight up the same anywhere
>> on the rail (section 1), and to swing the pole up the cart has to run
>> out along the rail, so the steps that score best can happen far from
>> the middle. Nothing in `r_tip` or `r_target_speed` minds the cart being
>> near an end, and hitting an end costs
>> nothing on that step: the episode just ends. So the robot learnt to
>> ride the cart into the ends[^20]: in one training run, 105 of 133
>> episodes ended at an end, and 352 of the 462 steps that scored above
>> 0.3 had the cart more than 0.2 m from the middle.
>>
>> `r_cart` is a nudge, not a rule:
>>
>> - **It can only cost so much.** Wherever the cart is, a step keeps at
>>   least two-thirds of its score (the floor, 3.2.1); with today's
>>   values, 0.737 at the very end of the rail. The tip score still
>>   decides most of what a step earns.
>> - **It never stops pulling.** From anywhere past the free zone, each
>>   bit closer to the middle pays a little more (3.2).
>>
>> With today's values, `c_z` = 0.050 m (3.1.1.4), `c_w` = 0.25 m (3.1.2)
>> and `cart_floor` = 0.667 (3.2.1):
>>
>> ```text
>> cart from the middle     c_d       exp(-c_d / c_w)   r_cart
>> 0                        0         1                 1
>> 0.03 m                   0         1                 1
>> 0.05 m (zone's edge)     0         1                 1
>> 0.1 m                    0.05 m    0.819             0.940
>> 0.2 m                    0.15 m    0.549             0.850
>> 0.3 m                    0.25 m    0.368             0.790
>> 0.44 m (the left end)    0.39 m    0.210             0.737
>> ```
>>
>> The last step of one episode in that run, with the cart riding into
>> the left end 0.434 m from the middle and the pole falling back from
>> near the top, scored 0.34. With `r_cart` it would score
>> 0.34 × 0.739 = 0.25.
>>
>>> ### 3.1 `exp(-c_d / c_w)`
>>>
>>> The same shrinking machine as `r_tip`'s (1.1): 1 when `c_d` is 0,
>>> 0.368 (about a third) one `c_w` further out, and 0.368 times smaller
>>> again for every further `c_w`, never 0. 1.1.4 explains `exp()`.
>>>
>>>> ### 3.1.1 `c_d`
>>>> ```text
>>>> c_d = max(0, |Cart − Cart Target| − c_z)
>>>> ```
>>>>
>>>> How far the cart is past the edge of its free zone, in metres. 0
>>>> anywhere inside the zone.
>>>>
>>>>> ### 3.1.1.1 `Cart`
>>>>>
>>>>> The cart's MuJoCo site, `16-Cart.Site`: where the cart is, at the
>>>>> moment of the capture. See its sub-section under *Where the
>>>>> reward's numbers come from*.
>>>>>
>>>>
>>>>> ### 3.1.1.2 `Cart Target`
>>>>>
>>>>> The middle's MuJoCo site, `15-Cart.Target.Site`: where Cart sits
>>>>> with the cart in the middle, which is where every episode starts.
>>>>> "The middle" in this section means this point. It is 2.5 mm left
>>>>> of halfway between the two ends: from it the cart has 0.440 m to
>>>>> the left base and 0.445 m to the right one (`11-Left-Cart.Site` and
>>>>> `13-Right-Cart.Site`).
>>>>>
>>>>
>>>>> ### 3.1.1.3 `|Cart − Cart Target|`
>>>>>
>>>>> How far the cart is from the middle, in metres, either way: the
>>>>> straight-line distance between the two points, worked out as in
>>>>> 1.1.1.3. Cart and Cart Target sit at the same height and the same
>>>>> front-to-back place, and the cart moves along the rail only, so
>>>>> `Δy` and `Δz` are always 0 and the distance is how far apart they
>>>>> are along the rail, with the sign dropped: left and right count the
>>>>> same.
>>>>>
>>>>> Cart 0.2 m left of the middle, Cart at (−0.210, 0.000, 0.050) and
>>>>> Cart Target at (−0.010, 0.000, 0.050):
>>>>>
>>>>> ```text
>>>>> 1. the differences:  Δx = −0.210 − (−0.010) = −0.200,  Δy = 0,  Δz = 0
>>>>> 2. square each:      (−0.200)² = 0.040,  0² = 0,  0² = 0
>>>>> 3. add them:         0.040 + 0 + 0 = 0.040
>>>>> 4. square root:      √0.040 = 0.200 m
>>>>> ```
>>>>>
>>>>
>>>>> ### 3.1.1.4 `c_z`
>>>>>
>>>>> The free zone's half-width: a distance, in metres, that we choose.
>>>>> It is set as `zone` in `trainer/gym.toml`'s `[reward.cart]`; today it is
>>>>> 0.050 m[^21], so the cart can wander 50 mm either way from the
>>>>> middle, 100 mm in all, without `r_cart` falling below 1.
>>>>>
>>>>
>>>>> ### 3.1.1.5 `max(0, … - c_z)`
>>>>>
>>>>> Taking `c_z` away counts the distance from the zone's edge instead
>>>>> of from the middle. Inside the zone that leaves a number below 0.
>>>>> `max(0, …)` means "whichever is bigger, 0 or this", so inside the
>>>>> zone `c_d` is 0, and `r_cart` is 1.
>>>>>
>>>>> ```text
>>>>> cart 0.03 m from the middle:  max(0, 0.03 − 0.05) = max(0, −0.02) = 0
>>>>> cart 0.2 m from the middle:   max(0, 0.2 − 0.05)  = max(0, 0.15)  = 0.15 m
>>>>> ```
>>>>>
>>>>
>>>
>>>> ### 3.1.2 `c_w`
>>>>
>>>> A distance, in metres, that we choose: how far past the zone's edge
>>>> the cart can go before the part of `r_cart` above the floor shrinks
>>>> to about a third. It is set as `width` in `trainer/gym.toml`'s `[reward.cart]`;
>>>> today it is 0.25 m[^21].
>>>>
>>>> It decides how far out the nudge reaches. The pull towards the middle
>>>> is how much `r_cart` changes as the cart moves: where `r_cart` is
>>>> still changing, moving towards the middle pays; where it has settled
>>>> near the floor, it barely does.
>>>>
>>>> `r_cart` for four values of `c_w`, with `c_z` = 0.050 m and
>>>> `cart_floor` = 0.667, the cart this far from the middle:
>>>>
>>>> ```text
>>>> c_w       0.05 m   0.1 m    0.2 m    0.3 m    0.44 m (the end)
>>>> 0.1 m     1.000    0.869    0.741    0.694    0.674
>>>> 0.25 m    1.000    0.940    0.850    0.790    0.737
>>>> 0.5 m     1.000    0.968    0.914    0.869    0.820
>>>> 1.0 m     1.000    0.984    0.954    0.926    0.892
>>>> ```
>>>>
>>>> - **Smaller `c_w`** (say 0.1 m): a strong pull just past the zone,
>>>>   then almost none. By 0.3 m the score is nearly at its floor, so
>>>>   moving between 0.3 m and the end barely changes it.
>>>> - **Larger `c_w`** (say 1.0 m): a gentle pull that keeps going all
>>>>   the way to the ends, but even at an end the cart costs only about a
>>>>   tenth of the score.
>>>>
>>>
>>>> ### 3.1.3 `-c_d / c_w`
>>>>
>>>> How many `c_w`s the cart is past the zone's edge, with a minus sign in
>>>> front, as in 1.1.3. Inside the zone it is 0. With the cart 0.2 m from
>>>> the middle it is −(0.15 ÷ 0.25) = −0.6.
>>>>
>>>
>>
>>> ### 3.2 `cart_floor + (1 - cart_floor) × …`
>>>
>>> Squeezes `exp()`'s range, from 1 down towards 0, into a range from 1
>>> down towards `cart_floor`[^22]. With the cart 0.2 m from the middle,
>>> where `exp(-c_d / c_w)` is 0.549:
>>>
>>> ```text
>>> 1. the room above the floor:  1 − 0.667      = 0.333
>>> 2. the share of it kept:      0.333 × 0.549  = 0.183
>>> 3. on top of the floor:       0.667 + 0.183  = 0.850
>>> ```
>>>
>>> - Inside the zone, `exp()` gives 1: 0.667 + 0.333 = 1.
>>> - Far out, `exp()` gives almost 0: 0.667 plus almost nothing, close
>>>   to 0.667 but never below it.
>>>
>>> **Why squeeze instead of cutting off at the floor.** A cut-off, a
>>> clamp that holds any number below 0.667 at 0.667, would leave the
>>> score flat from wherever `exp()` reaches 0.667 to the end of the rail,
>>> and nothing there would pull the cart back. Squeezed, `exp()` keeps
>>> shrinking all the way out, so from anywhere past the zone there is
>>> always a little more to gain by moving towards the middle.
>>>
>>>> ### 3.2.1 `cart_floor`
>>>>
>>>> The lowest `r_cart` can be, approached far out but never reached. It
>>>> is set as `floor` in `trainer/gym.toml`'s `[reward.cart]`; today it is 0.667[^21]:
>>>> wherever the cart is, a step keeps at least two-thirds of the score
>>>> the other scores earn.
>>>>
>>>> - **Higher** (say 0.9): a gentler nudge. Where the cart is can cost
>>>>   at most a tenth of the score.
>>>> - **Lower** (say 0.5, dm_control's[^22]): a firmer one, up to half.
>>>>
>>>
>>
>
>> ### 4 `r_reach`
>> ```text
>> r_reach = reach_floor + (1 - reach_floor) × exp(-d_r / reach_width)
>> ```
>>
>> The "tip above the middle of the rail" score[^24]. Its star, Target,
>> is fixed in the air where the tip is with the cart in the middle and
>> the pole straight up. On it, 1; the further the tip is from it, the
>> smaller, towards its floor, 0.667, never quite reaching it.
>>
>> It matters because the hold the robot is after is a pole standing
>> straight up with the cart in the middle. `r_tip` pays straight up the
>> same anywhere (section 1); `r_reach` adds a preference, from the
>> tip's side, for the place it stands: a hold in the middle earns more
>> than the same hold out along the rail. It is a gentle preference, not
>> the goal: wherever the tip is, a step keeps at least two-thirds of
>> its score.
>>
>> Pole straight up and still, `r_cart` and `r_reach` with today's
>> values (`r_tip`, `r_target_speed` and `r_speed` are 1):
>>
>> ```text
>> cart from the middle     r_cart   r_reach   r
>> 0                        1        1         1
>> 0.1 m                    0.940    0.869     0.817
>> 0.2 m                    0.850    0.790     0.671
>> 0.3 m                    0.790    0.741     0.585
>> 0.44 m (the left end)    0.737    0.704     0.519
>> ```
>>
>>> ### Does `r_reach` pull the pole into a lean?
>>>
>>> No, with today's values. Off the middle, leaning the pole towards
>>> the middle brings the tip closer to Target, so `r_reach` gains, but
>>> it takes the tip off the moving star, so `r_tip` loses. For the
>>> first bit of lean, `r_tip`'s loss is always the bigger, as long as
>>>
>>> ```text
>>> reach_width > w × (1 − reach_floor)
>>> ```
>>>
>>> today 0.2 m > 0.2 × 0.333 = 0.067 m. Searched lean by lean, from 0°
>>> to 40° in steps of 0.01°, with the pole still and the cart 0.05,
>>> 0.1, 0.2, 0.3 and 0.44 m from the middle, straight up paid the most
>>> every time: the opposite of the fixed star in section 1's table.
>>>
>>> Why the first bit of lean decides it: `r_tip` falls steeply right
>>> from the star (1.1: tilting by a small angle moves the tip 0.45 m ×
>>> the angle in radians, and every `w` of that costs about two-thirds
>>> of the score), while `r_reach` can only ever gain the share above its
>>> floor, `1 − reach_floor`, spread over `reach_width`.
>>>
>>> ### 4.1 `exp(-d_r / reach_width)`
>>>
>>> The same shrinking machine as `r_tip`'s (1.1) and `r_cart`'s (3.1):
>>> 1 on the star, 0.368 (about a third) one `reach_width` away, and
>>> 0.368 times smaller again for every further `reach_width`, never 0.
>>>
>>>> ### 4.1.1 `d_r`
>>>> ```text
>>>> d_r = |End − Target|
>>>> ```
>>>>
>>>> How far the pole's tip is from the fixed star, in metres, in a
>>>> straight line, worked out exactly as `d` in 1.1.1.3 but to Target,
>>>> `09-Pole.End.Target.Site`, instead of Dynamic Target. With the cart
>>>> centred the two stars are the same point, so `d_r` is `d`. Pole
>>>> straight up with the cart 0.2 m left: End at (−0.210, −0.132, 0.532)
>>>> and Target at (−0.010, −0.132, 0.532), so `d_r` = 0.200 m, while `d`
>>>> is 0.
>>>>
>>>
>>>> ### 4.1.2 `reach_width`
>>>>
>>>> A distance, in metres, that we choose: how far the tip can be from
>>>> the fixed star before the part of `r_reach` above the floor shrinks
>>>> to about a third. It is set as `width` in `trainer/gym.toml`'s `[reward.reach]`;
>>>> today it is 0.2 m[^24], the same as `r_tip`'s `w` to start, kept
>>>> apart so the two can be tuned separately. Below 0.067 m today, a
>>>> lean would pay (*Does `r_reach` pull the pole into a lean?*).
>>>>
>>>
>>> ### 4.2 `reach_floor + (1 - reach_floor) × …`
>>>
>>> The same squeeze as `r_cart`'s (3.2): `exp()`'s range, from 1 down
>>> towards 0, squeezed into a range from 1 down towards `reach_floor`.
>>> With the tip 0.2 m from the star, where `exp(-d_r / reach_width)` is
>>> 0.368:
>>>
>>> ```text
>>> 1. the room above the floor:  1 − 0.667      = 0.333
>>> 2. the share of it kept:      0.333 × 0.368  = 0.123
>>> 3. on top of the floor:       0.667 + 0.123  = 0.790
>>> ```
>>>
>>>> ### 4.2.1 `reach_floor`
>>>>
>>>> The lowest `r_reach` can be, approached far from the star but never
>>>> reached. It is set as `floor` in `trainer/gym.toml`'s `[reward.reach]`; today it
>>>> is 0.667[^24]: wherever the tip is, a step keeps at least two-thirds
>>>> of the score the other scores earn.
>>>>
>>>> - **Higher** (say 0.9): a gentler preference for the middle.
>>>> - **Lower** (say 0.5): a firmer one, but the lean rule above gets
>>>>   stricter: `reach_width` must then be more than 0.1 m.
>>>>
>>>
>>
>
>> ### 5 `r_speed`
>> ```text
>> r_speed = speed_floor + (1 - speed_floor) × exp(-over_speed / speed_width)
>> ```
>>
>> The "never turn much too fast" score[^27]. It watches the pole's turning
>> speed all the way round the circle, with no zone. At or below
>> `speed_limit`, wherever the pole is, it is exactly 1. Above it, the score
>> shrinks the faster the pole goes, towards its floor, 0.250, but never
>> quite reaches it.
>>
>> It matters because a pole can be whipped round far faster than any
>> swing-up needs. A swing-up passes the bottom at 10.5 rad/s (2.1.1.3); the
>> fastest pole recorded in training reached 33.37 rad/s, End moving at about
>> 15 m/s. A pole going that fast is over the limit around most of its turn,
>> including the upper half, where the tip earns most of its score, so the
>> cut reaches the steps that pay.
>>
>> With today's values, `speed_limit` = 12 rad/s (5.1.1.2), `speed_width` =
>> 1 rad/s (5.1.2) and `speed_floor` = 0.250 (5.2.1):
>>
>> ```text
>> speed       End speed   over_speed   r_speed
>> 12 rad/s    5.4 m/s     0            1
>> 13 rad/s    5.9 m/s     1            0.526
>> 14 rad/s    6.3 m/s     2            0.352
>> 15 rad/s    6.8 m/s     3            0.287
>> 20 rad/s    9.0 m/s     8            0.250
>> 33 rad/s    14.9 m/s    21           0.250
>> ```
>>
>> Inside the zone (2.1.2) a step over this limit also takes
>> `r_target_speed`'s cut, and the two multiply: at the floors, 0.667 × 0.250
>> = 0.167.
>>
>>> ### 5.1 `exp(-over_speed / speed_width)`
>>>
>>> The same shrinking machine as `r_tip`'s (1.1): 1 when `over_speed` is 0,
>>> 0.368 (about a third) one `speed_width` past the limit, and 0.368 times
>>> smaller again for every further `speed_width`, never 0. 1.1.4 explains
>>> `exp()`.
>>>
>>>> ### 5.1.1 `over_speed`
>>>> ```text
>>>> over_speed = max(0, |speed| − speed_limit)
>>>> ```
>>>>
>>>> How far past the limit the pole is going, in rad/s. 0 is the good case:
>>>> there is nothing to charge, and `r_speed` is 1.
>>>>
>>>>> ### 5.1.1.1 `|speed|`
>>>>>
>>>>> The pole's turning speed, straight from MuJoCo (2.1.1.1), with its sign
>>>>> dropped: turning left and turning right count the same.
>>>>>
>>>>
>>>>> ### 5.1.1.2 `speed_limit`
>>>>>
>>>>> The speed, in rad/s, up to which the pole is not charged anywhere. We
>>>>> choose it. It is set as `limit` in `trainer/gym.toml`'s
>>>>> `[reward.speed]`; its starting value is 12 rad/s[^27].
>>>>>
>>>>> To picture it: multiply by the 0.45 m from the hinge to End, and 12 rad/s
>>>>> is End moving at about 12 × 0.45 = 5.4 m/s. As in 2.1.1.3, that is a
>>>>> rough guide for the eye, not an exact figure.
>>>>>
>>>>> ```text
>>>>> speed_limit   End speed   what it means
>>>>> 10.5 rad/s    4.7 m/s     a perfect swing-up's speed at the bottom
>>>>> 11 rad/s      5.0 m/s     just above a perfect swing-up
>>>>> 12 rad/s      5.4 m/s     today's: room for an imperfect swing-up
>>>>> 13 rad/s      5.9 m/s     further above any swing-up
>>>>> ```
>>>>>
>>>>> - **Lower** (say 11 rad/s): charges more spinning, but leaves a swing-up
>>>>>   little room for error at the bottom.
>>>>> - **Higher** (say 15 rad/s): more room, but more fast spinning goes
>>>>>   uncharged.
>>>>>
>>>>
>>>
>>>> ### 5.1.2 `speed_width`
>>>>
>>>> A speed, in rad/s, that we choose: how far past the limit the pole can
>>>> go before the part of `r_speed` above the floor shrinks to about a third.
>>>> It is set as `width` in `trainer/gym.toml`'s `[reward.speed]`; its
>>>> starting value is 1 rad/s[^27]. It is `r_speed`'s own, kept apart from
>>>> `target_width` so the two can be tuned separately.
>>>>
>>>> `r_speed` at each speed, with `speed_limit` = 12 rad/s and `speed_floor`
>>>> = 0.250:
>>>>
>>>> ```text
>>>> speed_width   13 rad/s   14 rad/s   15 rad/s   20 rad/s
>>>> 0.5 rad/s     0.352      0.264      0.252      0.250
>>>> 1 rad/s       0.526      0.352      0.287      0.250
>>>> 2 rad/s       0.705      0.526      0.417      0.264
>>>> ```
>>>>
>>>
>>>> ### 5.1.3 `-over_speed / speed_width`
>>>>
>>>> How many `speed_width`s the pole is past the limit, with a minus sign in
>>>> front, as in 1.1.3. At or below the limit it is 0. At 14 rad/s it is
>>>> −(2 ÷ 1) = −2.
>>>>
>>>
>>
>>> ### 5.2 `speed_floor + (1 - speed_floor) × …`
>>>
>>> The same squeeze as `r_cart`'s (3.2): `exp()`'s range, from 1 down
>>> towards 0, squeezed into a range from 1 down towards `speed_floor`. At
>>> 14 rad/s, where `exp(-over_speed / speed_width)` is 0.135:
>>>
>>> ```text
>>> 1. the room above the floor:  1 − 0.250      = 0.750
>>> 2. the share of it kept:      0.750 × 0.135  = 0.102
>>> 3. on top of the floor:       0.250 + 0.102  = 0.352
>>> ```
>>>
>>>> ### 5.2.1 `speed_floor`
>>>>
>>>> The lowest `r_speed` can be, approached at high speed but never reached.
>>>> It is set as `floor` in `trainer/gym.toml`'s `[reward.speed]`; its
>>>> starting value is 0.250[^27]: however fast the pole turns, a step keeps at
>>>> least a quarter of the score the other scores earn. It is firmer than the
>>>> other floors because a pole this fast is never what the robot needs.
>>>>
>>>> - **Higher** (say 0.5): a gentler charge, up to half.
>>>> - **Lower** (say 0.1): a firmer one, up to nine-tenths.
>>>>
>>>
>>
>

## References

Where each idea and borrowed value comes from, checked against its
source on 2026-10-28. "Simulation" or "real hardware" says where the
source used it. Nothing borrowed here has been tested on this rig yet.

[^1]: **MuJoCo sites.** MuJoCo's documentation: "Sites are essentially
    light geoms. They represent locations of interest within the body
    frame." They take no part in collisions or in the model's mass
    ([MuJoCo Overview, *Site*](https://mujoco.readthedocs.io/en/stable/overview.html)).
    This rig's twelve are named in `trainer/settings.toml` `[mjcf]`.

[^2]: **Scoring from the simulator while the robot sees only the
    camera.** A project rule, not a published result: training happens
    only in the simulation, so the reward never has to be worked out on
    the real robot (`AGENTS.md`, *Not a toy* and *The real robot*), and
    every reward value comes from the sites (user, 2026-10-25;
    `synthetic-loop-dia.md`, *Contracts*, the sites) and the pole's own
    numbers from MuJoCo (user, 2026-10-28; *Contracts*, the pole and
    the run's values). DeepMind's
    cart-pole likewise scores from the simulator's own state
    (`physics.pole_angle_cosine()`, `physics.angular_vel()`;
    [dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
    Simulation.

[^3]: **ArUco markers.** The four printed markers and their dictionary
    are described in `ArUco.md` in this repo; the detector that finds
    them is the trained YOLO model (`AGENTS.md`, *The trained detector*).

[^4]: **A reward made of scores multiplied together.** DeepMind's
    cart-pole reward is `upright.mean() * small_control * small_velocity
    * centered`
    ([dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)),
    and the Control Suite paper keeps every reward "in the unit
    interval"
    ([Tassa et al. 2018, section 2](https://ar5iv.labs.arxiv.org/html/1801.00690)).
    Simulation.

[^5]: **A reward that is more than 0 and at most 1.** Muratore et al.
    score a real cart-pole with `exp(−(sᵀQs + aᵀRa))`: "the reward is in
    range ]0,1] for every time step" (`]0,1]` is the same range as
    `0 < r ≤ 1`)
    ([Muratore et al. 2019, appendix A, Eq. 40](https://ar5iv.labs.arxiv.org/html/1907.04685)).
    Real hardware (a Quanser cart-pole).

[^6]: **The tip score.** The cloud-edge DDPG rig's reward starts with
    `e^(−δ·d)`, where `d` is the "Cartesian Euclidean distance … between
    the tip of the pole and the target position", the target being the
    tip's position with the cart centred and the pole upright
    ([Cao et al. 2022, section IV-A2, Eq. 9](https://ar5iv.labs.arxiv.org/html/2203.02230)).
    Real hardware (a Quanser pendulum). The paper does not say its
    episodes started with the pole hanging.

[^7]: **Pythagoras' rule in 3D.** The distance between two points is
    √((p₁ − q₁)² + (p₂ − q₂)² + (p₃ − q₃)²), worked out "using the
    Pythagorean theorem"
    ([Wikipedia, *Euclidean distance*](https://en.wikipedia.org/wiki/Euclidean_distance)).

[^8]: **`w` = 0.2 m.** From the same DDPG rig (note 6): its `δ` is 5
    (Table I), and `exp(−5 × d)` is `exp(−d / 0.2)`, because dividing by
    0.2 is multiplying by 5. Chosen for that rig's pendulum, not this
    one's.

[^9]: **e and `exp()`.** e is "the base of the natural logarithm and
    exponential function", the limit of (1 + 1/n)ⁿ, found by Jacob
    Bernoulli in 1683 while working on compound interest
    ([Wikipedia, *e (mathematical constant)*](https://en.wikipedia.org/wiki/E_(mathematical_constant))).
    The trainer uses Python's `math.exp`, "e raised to the power x"
    ([Python `math`](https://docs.python.org/3/library/math.html)).

[^10]: **Why `r_target_speed` exists.** This project's first training run
    learned to spin the pole, because each turn sweeps the tip past the
    star (`trainer-observations.md`, *What the policy actually does: it
    spins the pole*, and *Why spinning is a trap*). Every reward tried
    since has ended in spinning too (user, 2026-10-28). Simulation.

[^12]: **Moving energy and height energy.** A turning body's moving
    energy is `½ × I × ω²`
    ([Wikipedia, *Rotational energy*](https://en.wikipedia.org/wiki/Rotational_energy));
    lifting a mass `m` by a height `h` takes `m × g × h`
    ([Wikipedia, *Gravitational energy*](https://en.wikipedia.org/wiki/Gravitational_energy)).
    Here `ω` is `speed` (2.1.1.1) and `h` is the height still to climb,
    `l_c × (1 − cos(lean))`. Dividing both by `m × g × l_c`, and using
    `ω0² = m × g × l_c ÷ I` (2.1.1.4), gives the line in 2.1.1.3. It
    assumes the pole's centre of mass lies on the line from the hinge to
    End.

[^13]: **The pole's speed, centre of mass and hinge point, straight from
    MuJoCo.** MuJoCo keeps each joint's velocity in `qvel` ("velocity"),
    works out each group of parts' centre of mass in `subtree_com`
    ("center of mass of each subtree") and each joint's point in
    `xanchor` ("Cartesian position of joint anchor"), all in its
    `mjData` (`include/mujoco/mjdata.h` of the 3.15.0 release). The
    physics crate reads them at every capture (`physics/README.md`,
    `pole()`), and the orchestrator sends them with the sites
    (`synthetic-loop-dia.md`, *Contracts*, the pole; user, 2026-10-28).
    The reward reads only the speed. Simulation.

[^14]: **The natural frequency.** A pole swinging on a hinge is a
    compound pendulum; its small-swing period is
    `T = 2π √(I / (m × g × l_c))`
    ([Wikipedia, *Pendulum (mechanics)*, *Compound pendulum*](https://en.wikipedia.org/wiki/Pendulum_(mechanics))),
    and `ω0 = 2π ÷ T` gives `ω0 = √(m × g × l_c ÷ I)`.

[^15]: **The pole's mass, centre of mass and inertia.** MuJoCo's own
    values, read by the physics crate when it loads the MJCF the run
    names (`trainer/settings.toml` `[mjcf] model`) and sent once per run
    in the run's values (`synthetic-loop-dia.md`, *Contracts*): the mass
    is MuJoCo's `body_subtreemass` of the hinge's body; `l_c` is the
    centre of mass's distance from the hinge's axis; `I` is the hinge's
    entry in MuJoCo's inertia matrix. The crate refuses to load if they
    disagree with MuJoCo's other figures (`physics/README.md`, check 7).
    On the run's model (the MJCF of 2026-10-27), MuJoCo gives 0.402 kg,
    0.155 m and 0.0222 kg·m², printed in the physics crate's load line
    (2026-10-28). Run 3's free swings measured 5.0 rad/s against the 5.25 worked out here;
    the hinge's friction (note 16) and the cart's motion slow a real
    swing.

[^16]: **The hinge's friction.** The MJCF gives `04-Pole.Hinge.joint`
    `frictionloss="0.00018"`, `damping="0"` (user, 2026-10-29; 0.02
    before). Fitted to the rig: let go from horizontal with the cart held,
    the real pole made 300 swings before it stopped, and at 0.00018 the
    model's pole made 303 with the cart left to the model's motor, as in
    training (`trainer/src/pole_drop.py` as first written). The script
    now freezes the cart, as the user asked: frozen, 0.00018 gives 1,734
    swings and 0.00135 gives 300, because the model's cart, pushed about
    by the pole, takes energy the real held cart does not. MuJoCo's
    reference:
    "Friction loss due to dry friction"
    ([MuJoCo XML reference, `joint/frictionloss`](https://mujoco.readthedocs.io/en/stable/XMLreference.html#body-joint-frictionloss)).
    Gravity is MuJoCo's default, `"0 0 -9.81"`
    ([`option/gravity`](https://mujoco.readthedocs.io/en/stable/XMLreference.html#option-gravity));
    the MJCF's `<option>` does not set it.

[^19]: **A score for where the cart is, with a free zone.** dm_control's
    cart-pole scores the cart's position:
    `centered = rewards.tolerance(physics.cart_position(), margin=2)`
    ([dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
    Its `tolerance()` is 1 inside a band, its `bounds`, the "target
    interval", and falls off outside it over a distance, its `margin`
    ([dm_control `rewards.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/utils/rewards.py)).
    The free zone here plays the part of `bounds`. dm_control's
    cart-pole uses no band (its `bounds` are left at 0) and falls off as
    a bell; this score falls off as `r_tip`'s `exp()` (note 6) instead.
    Read 2026-10-29. Simulation.

[^20]: **Why `r_cart` exists.** Training run `trainer/runs/20261028_235359`,
    on an earlier reward, measured 2026-10-29 from its
    `episodes.csv` (episodes 500 to 620: 105 of 133 ended at a strike
    gap) and from the last 34,000 steps of its captures (352 of 462
    steps above 0.3 had the cart more than 0.2 m from the middle; the
    episode ridden into the left end is episode 616). `trainer/runs/` is
    gitignored, so these logs are not in the repo. Simulation.

[^21]: **`c_z` = 0.050 m, `c_w` = 0.25 m and `cart_floor` = 0.667.** No
    published source for any of them. The free zone and the floor are
    the user's (2026-10-29), for a nudge towards the middle, not a rule.
    `c_w` is a starting value to tune, picked so that the nudge still
    pulls at the ends (the table in 3.1.2).

[^22]: **Squeezing a score above a floor.** dm_control's cart-pole lifts
    its scores off 0 the same way: `centered = (1 + centered) / 2` keeps
    `centered` between 0.5 and 1, and `small_control = (4 + small_control) / 5`
    keeps `small_control` between 0.8 and 1
    ([dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
    Those are `cart_floor + (1 - cart_floor) × score` with `cart_floor`
    0.5 and 0.8. Read 2026-10-29. Simulation.

[^23]: **Why the star moves with the cart.** The user's (2026-10-29),
    after reading that the fixed star pays a lean off the middle. The
    table in section 1 is worked from the rig's numbers, not measured
    in a run: End 0.45 m from the hinge, the pole still and `w` = 0.2 m,
    with the best lean the one pointing the tip at the star. DeepMind's
    cart-pole likewise scores the pole being upright apart from where
    the cart is: `upright = (physics.pole_angle_cosine() + 1) / 2`, from
    the pole's angle alone, and `centered` from the cart's position
    ([dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
    Simulation.

[^24]: **`r_reach`, `reach_width` = 0.2 m and `reach_floor` = 0.667.**
    The user's (2026-10-29): the fixed star's score kept as a factor
    between 0.667 and 1, with its own width, starting at `r_tip`'s `w`.
    No published source. The lean rule and its search in section 4 are
    worked from the rig's numbers, not measured in a run.

[^25]: **`r_target_speed` and its values.** The user's (2026-10-30): a penalty only
    above a speed and only in a zone, shrinking by `exp()`, the zone
    measured along the rail alone and the tip above Pivot. The nearest
    published score is DeepMind's `small_velocity`, `(1 + tolerance(angular_vel,
    margin=5)) / 2`, which scores the speed everywhere and floors at 0.5
    ([dm_control `cartpole.py`](https://raw.githubusercontent.com/google-deepmind/dm_control/main/dm_control/suite/cartpole.py)).
    No published source was found for scoring speed only near the top.
    `target_limit` = 0.5 rad/s is the user's (2026-10-30), from End's speed
    in the table in 2.1.1.3, and so is `target_floor`, configurable like
    `r_cart`'s and `r_reach`'s, at 0.667. `zone_dx` = 0.154 m and
    `target_width` = 1 rad/s are starting values, to tune; `zone_dx` comes
    from the 20° band of the training measures. None has a published source.

[^26]: **What recorded play earns.** Measured 2026-10-30 from the capture
    record of `trainer/runs/20261029_222437`, played under another reward.
    Slower spins are episodes 839, 1200, 1234, 1240 and 1250 (212.8, 116.4,
    137.6, 118.8 and 123.0); the ten fastest spins, by mean speed, are
    episodes 786, 735, 728, 877, 836, 841, 793, 820, 737 and 818 (46.6, 35.8,
    40.8, 49.4, 43.3, 43.9, 48.3, 40.4, 36.2 and 42.0); hanging about is
    episode 1262 (12.3). Mean speed
    on the passes over the top in episodes 1200, 1240 and 1250: 2.88, 5.86
    and 3.53 rad/s. Scored from the record's `tip`, `cart` and `reach`
    columns, `pole_speed`, `end_x`, `end_z`, `dynamic_target_x` and
    `pivot_z`. `trainer/runs/` is gitignored, so these logs are not in the
    repo, and the scripts were scratch files. Simulation.

[^27]: **`r_speed` and its values.** The user's (2026-10-30): a charge for
    turning much too fast anywhere, with no zone and a configurable floor
    starting at 0.250. `speed_limit` = 12 rad/s was agreed the same day, from
    the fastest speeds recorded, MuJoCo's own: 30.81, 30.36, 30.25 and
    33.37 rad/s in `trainer/runs/20261028_235359`, `20261029_024604`,
    `20261029_170036` and `20261029_222437`, with 10.8%, 10.0%, 17.1% and
    6.3% of their captures above 12 rad/s. `speed_width` = 1 rad/s is a
    starting value, to tune. None has a published source. Simulation.
