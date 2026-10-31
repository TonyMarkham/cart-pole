# Calibrate with a homography — cart position and pole angle from the camera

The robot has to know where its cart is, and it has no sensor that says
so: the stepper reports nothing, and the camera sees only four marker
centres in pixels. This plan turns those pixels into metres along the
rail, and the pole's angle, with one matrix built once during the rig's
existing calibration and applied to every frame after.

Status: **plan** (user, 2026-10-31). Not built. It is a new scheme, on
the `feature/dm_control` branch: the policy reads the cart's position and
the pole's angle, as DeepMind's cart-pole does
(`trainer/dm_control_reward.md`, *Where the reward's numbers come from*),
derived on the Orin from the detections. It stands beside the synthetic
loop (`synthetic-loop-dia.md`) and changes nothing in it; how the two
meet is not designed here.

## The problem

The detector finds the four ArUco markers in each 728×544 frame and
returns each one's centre as a pixel position `(u, v)`: `u` across the
image, `v` down it. It does this to under a pixel (user, 2026-10-31, on
the real Orin, moving pole included). At 2.41 m from the rig the frame
spans about 2.0 m, so a pixel is about 2.7 mm[^1].

A pixel position is not a position on the rail. The camera is a
perspective projection: equal metres on the rig are not equal pixels in
the image unless the camera is exactly square to the rail, and the two
fixed markers, Left and Right, cannot tell. They are two points on one
line, and two points fix only a scale along that line. Worse, they sit
in a different plane from Pivot and End, 55 mm away in depth, so even
their scale is 2.3% off for the pole[^2]. With the detector under a
pixel, these systematic errors are the whole error: 10 mm from the
depth alone at the rail's end, and an unknown amount from the camera's
angle.

A homography fixes both. It is the exact map from a flat plane in the
world to the image for any camera position and angle, lens distortion
aside (*What it does not cover*). The pole's plane is such a plane:
Pivot and End always lie in it, whatever the cart and pole do, because
the cart runs along a straight rail and the pole swings in one flat
plane.
One matrix maps that plane; fitting it needs four or more points in the
plane whose positions are known in both pixels and metres, no three on
a line.

## Terms

- **The robot's frame of reference:** the MJCF's world frame, in which
  every site is given (`trainer/reward-plan.md`, *Where the reward's
  numbers come from*): `x` along the rail, `z` up, `y` front to back,
  in metres. Cart position here is Pivot's `x` relative to the centre
  stop (*The three stops*), so it is 0 with the cart centred, negative
  to the left as the camera sees it (`robot-plan.md`, *Terms*: a
  positive command moves the cart towards `+x`, right in the camera's
  frame).
- **The pole's plane:** the vertical plane `y = −0.132` that Pivot and
  End move in. Positions in it are `(x, z)`.
- **The base plane:** the vertical plane `y = −0.077` that Left and
  Right sit in, 55 mm from the pole's plane. Nothing in it is mapped.
- **Pixel position `(u, v)`:** the detector's box centre, in pixels of
  the 728×544 frame, `(0, 0)` at the top left. `v` grows downwards, the
  opposite of `z`.
- **Homography:** a 3×3 matrix `H` that maps a point on one plane to its
  image on another, here the pole's plane to the camera's picture. It
  has eight free numbers (the ninth is a scale that cancels), so four
  point pairs, no three in a line, fix it, and more pairs let it be
  fitted by least squares and checked.
- **Least squares:** choosing the matrix that makes the total of the
  squared errors over all the point pairs as small as possible. With
  exactly four pairs the fit is exact and tells nothing; with more, the
  leftover errors, the **residuals**, say how well the matrix explains
  the points, and so how good it is.
- **Sub-pixel:** a position known to a fraction of a pixel. A box centre
  is the middle of a box whose edges the detector places between pixel
  columns, so it can be finer than the pixel grid.

## What is known: the ground truth

The real-world side of each point pair comes from the rig's geometry,
never measured by the camera. Two sources, both the rig's CAD
(`freecad/`), the MJCF being exported from it:

- **The sites**, from the MJCF, with the cart centred and the pole
  hanging (`trainer/reward-plan.md`, *Where the reward's numbers come
  from*): Pivot `(−0.010, −0.132, 0.082)`, End `(−0.010, −0.132,
  −0.368)`, 0.450 m straight below it; Left `(−0.510, −0.077, 0.025)`,
  Right `(0.495, −0.077, 0.025)`. Pivot's height, `z_P` = 0.082 m, never
  changes; End hanging is always at `z_P − 0.450`.
- **The three stops** of the Arduino's Calibrate (`calibrate/README.md`):
  the cart stands on the first limit switch, then on the second, then
  exactly at the centre, halfway between the two trip points. Where the
  switches trip is the one thing the CAD knows only in theory: the
  physical build varies, so it is measured on the rig (user,
  2026-10-31), as the two gaps below.

**The stops' `x`, from two hand measurements and three MJCF values**
(user, 2026-10-31). With the cart stopped on each switch, a human
measures the gap between the cart's face and the wall of the rail
support it faces, about 5 mm:

- `gap_left`: the stopped cart's face to the left support's wall.
- `gap_right`: the stopped cart's face to the right support's wall.

These are the human's entries in the calibration's toml (*Settings*).
The rest would be hard to measure by hand and comes from the MJCF, each
value citing its site:

```text
left support wall, x               10-Left-Rail.Site                                  −0.485
right support wall, x              12-Right-Rail.Site                                  0.470
cart's left face to Pivot centre   Pivot x − 11-Left-Cart.Site x = −0.010 − (−0.045)   0.035
cart's right face to Pivot centre  13-Right-Cart.Site x − Pivot x = 0.025 − (−0.010)   0.035
```

The two face-to-Pivot values happen to be equal in the MJCF; they are
two sites and two settings all the same. The stops' `x` are worked out,
never entered, each side from its own gap and its own sites:

```text
x_left_stop  = left_wall_x  + gap_left  + left_face_to_pivot
x_right_stop = right_wall_x − gap_right − right_face_to_pivot
x_centre     = (x_left_stop + x_right_stop) / 2
```

The gaps need not match. With `gap_left` 0.005 m and `gap_right` 0.007 m:

```text
x_left_stop  = −0.485 + 0.005 + 0.035 = −0.445
x_right_stop =  0.470 − 0.007 − 0.035 =  0.428
x_centre     = (−0.445 + 0.428) / 2   = −0.0085
```

A millimetre of error in a gap is a millimetre in the matrix's ruler.
The centre is halfway between the trip points, which is Calibrate's own
definition of it. Calibrate's span, in pulses, times the metres per
pulse `trainer/settings.toml` `[drive]` defines (belt travel per turn
over steps per turn) must equal `x_right_stop − x_left_stop`: the check
on the two gaps.

Call the three stops' Pivot `x` values `x_A`, `x_B` (the two switches, in
the order Calibrate reaches them; the board reports which pin tripped,
so which is left and which right is known) and `x_C` (the centre).

## The calibration sequence

The camera calibration rides on the Arduino's Calibrate (user,
2026-10-31): the cart's three stops are the moments the cart's position
is known, and the Orin measures at each. **The Orin runs the stages**
(user, 2026-10-31): it tells the Arduino when to make each move, so it
always knows which stop the cart stands on, and no stop ends before the
Orin has measured it. No one touches the rig.

1. **Start.** The Orin begins capturing frames and detecting, at the
   loop's rate, as it does in a run, keeping each frame's four slots
   with the frame's time. It then triggers Calibrate's first move.
2. **Stop A, the first switch.** The cart seeks slowly to a switch and
   stops; the Arduino reports the trip and waits. Pivot is still; End
   swings a little, the seek being slow, and keeps swinging, since the
   hinge takes hundreds of swings to stop (`trainer/reward-plan.md`,
   note 16). The Orin measures Pivot and End (*The measurement at a
   stop*), then triggers the next move. Two known points: Pivot
   `(x_A, z_P)`, End `(x_A, z_P − 0.450)`.
3. **Stop B, the second switch.** The cart seeks slowly to the other
   switch, counting pulses, and waits. The same measurement, then the
   trigger for the move to the centre. Pivot `(x_B, z_P)`, End
   `(x_B, z_P − 0.450)`.
4. **Stop C, the centre.** The cart moves to the centre at seek speed,
   ramped as the seeks are, so the pole is left swinging as little as
   it is at the switches (user, 2026-10-31: a slow centring move, to
   limit the wobble). The same measurement. Pivot `(x_C, z_P)`, End
   `(x_C, z_P − 0.450)`.
5. **Left and Right.** Measured as Pivot is, over all three stops, for
   the camera-moved check (*Per frame*), never for the fit.
6. **Fit.** Six point pairs, each a pixel position and a metre position
   in the pole's plane:

   ```text
   pair         pixels (u, v)                    metres (x, z)
   Pivot A      clipped mean at stop A           (x_A, 0.082)
   Pivot B      clipped mean at stop B           (x_B, 0.082)
   Pivot C      clipped mean at stop C           (x_C, 0.082)
   End A        the curve's vertex at stop A     (x_A, −0.368)
   End B        the curve's vertex at stop B     (x_B, −0.368)
   End C        the curve's vertex at stop C     (x_C, −0.368)
   ```

   The three Pivots lie on one line, the rail; the three Ends lie on a
   line below it. A fit needs four points with no three on a line, and
   has them in any two Pivots and any two Ends; the other two points
   overdetermine it, which is what makes the residuals a check (with
   exactly four points the fit is exact and every residual is 0). The
   centre's Pivot is the most valuable of the six for the cart's
   position: a known point on the rail itself, halfway along, where
   the cart spends most of its time, and a still marker's mean, the
   most precise kind of point there is.

   `cv2.findHomography(pixels, metres, method=0)` fits `H`, pixels to
   metres, by least squares[^3]. Then each metre point is mapped back
   to pixels through the inverse and compared with its measured pixel
   position: the residuals. Every residual under the threshold
   (*Settings*; proposed 1 px) accepts the fit; one over it fails the
   calibration, and the calibration is run again. One residual has a
   meaning of its own: a vertex is the true bottom of the pole's circle
   only for a camera square to the rail, drifting off it with the
   camera's angle and the swing's width (*The measurement at a stop*),
   so a vertex residual out of line with the Pivots' says the camera is
   not square. The fit is then repeated without that vertex, and the
   drift is recorded with the matrix.
7. **Store.** The nine numbers of `H`, the residuals, the six pairs
   with each point's detection count, spread and number dropped, Left's
   and Right's measured positions, the frame size the pixels refer to
   (728×544), and the date, in one file on the Orin at the path the
   settings name. The Orin loads it at start; a run with no stored
   matrix does not start, as Run needs a finished Calibrate
   (`robot-plan.md`, *The Arduino's page*).

### The measurement at a stop

Turns a stop's many detections of one marker into one point, trusting
no single detection (user, 2026-10-31).

- **At least 30 detections of each marker the stop needs**, however
  long that takes: Pivot and End at a switch, Pivot at the centre. The
  stop does not end before, so the calibration cannot proceed on thin
  data. A marker that cannot reach 30, hidden or missed by the detector,
  fails the calibration.
- **Then the configured pause, `window_s`**, counted from the trigger,
  over which every further detection is taken too. 30 is a floor, not a
  target: a longer pause gives a more accurate point, because the noise
  in a point made from `n` detections shrinks as `1 ÷ √n`.
- **Pivot's point: a clipped mean, not a plain one.** Pivot is still,
  so its detections scatter about one place, but the detector can
  return a wrong box now and then, and one detection 20 px off pulls a
  plain mean of 30 by 0.7 px, more than the detector's own error. So:
  take the centre of the detections (their median, the middle value,
  which a far-off point cannot drag), their spread about it (their
  standard deviation, the usual measure of how widely values scatter),
  drop every detection further from the centre than `clip_k` spreads
  (proposed 3), and take the plain mean of the rest. The count, the
  spread and the number dropped are stored with the point.
- **End's point: the vertex of a curve fitted to its detections.** End
  is swinging, so its detections do not scatter about one place: they
  lie along a curve, the camera's view of the pole's circle about
  Pivot. In the pole's plane that is a true circle of radius 0.450 m;
  in the image it is whatever the perspective makes of it, and it is
  called a curve here for that reason. The point the fit needs is where
  the pole hangs straight down, `(x, z_P − 0.450)` in metres, and in
  the image that is the curve's lowest point, its **vertex**: where the
  curve turns. It is read off the curve's shape, so it does not depend
  on how long End was watched or where in its swing the pause began
  and ended. It is never a mean of the detections, and never the
  midpoint between the ends of the observed curve, both of which would
  depend on exactly that.

  Near its bottom a circle is a parabola to high accuracy, so the curve
  is a parabola, `v = a × (u − u₀)² + v₀`, fitted by least squares to
  the detections; its vertex is `(u₀, v₀)`, three numbers from 30 or
  more points. How far the real curve strays from a parabola grows with
  the swing's width, and the slow moves keep it small:

  ```text
  swing, each side of the bottom   pole angle   parabola error at the ends
  2 cm                             2.5°         0.0002 mm
  10 cm                            12.8°        0.14 mm
  20 cm                            26.4°        2.2 mm
  ```

  (The error is about `a⁴ ÷ (8 × 0.450³)` for a swing of `a` each
  side.) Up to about 10 cm each side the error is under a tenth of a
  pixel and the parabola serves; a swing of 20 cm each side would want
  a general second-degree curve instead, five numbers, which follows
  any perspective view of a circle exactly.
- **End's clipping is measured against its curve, not against a
  centre.** End has no still centre: its detections lie along the
  curve, so the reference is the curve itself. The steps, in order:

  1. Fit the parabola to all the detections.
  2. For each detection, its **error**: the shortest distance from the
     detection to the parabola, in pixels, positive on one side of the
     curve and negative on the other.
  3. The **spread**: the standard deviation of those errors, over all
     the detections.
  4. Drop every detection whose error, sign ignored, is more than
     `clip_k` × the spread.
  5. Fit the parabola again to the detections that remain. Its vertex
     is End's point.

  Stored with the vertex: the number of detections, the spread of step
  3, and the number dropped in step 4. Pivot's clipping is the same
  rule with a different reference: Pivot's errors are distances from
  its median centre, End's are distances from its curve.
- **What the vertex assumes.** The image's lowest point is the true
  bottom of the pole's circle exactly when the camera is square to the
  rail. With an oblique camera the two part, by an amount that grows
  with the camera's angle and the swing's width. At a couple of
  centimetres of swing that is far under the detector's error; the
  residual check in step 6 is where a larger one would show.

Follows from rules already set:

- **Calibrate's sequence is this one.** Three slow moves, each on the
  Orin's trigger, each ending in a wait for the next: seek to a switch,
  seek to the other, move to the centre at seek speed. In the page's
  pattern (`calibrate/README.md`, *The page*), each move is a POST the
  Orin sends, so loading the page still moves nothing, and the board's
  state names the stop it stands on.
- **The metre values are settings, not code** (`AGENTS.md`, *Tunable
  values live in a toml*): `x_A`, `x_B`, `x_C`, `z_P` and the pole's
  0.450 m all come from the toml, so a rebuilt rig changes the file,
  not the program.
- **The pause, the 30 floor, `clip_k` and the residual threshold are
  settings** too.

Proposed, not decided:

- **The calibration is a mode of the Orin's program**, beside
  `WebSocketServer` and `RobotBrain` (`robot-plan.md`, *The Orin's
  modes*), started from the Orin's page.
- **The calibration's record is kept**, every frame's slots and times
  and the derived points, so a bad fit can be read afterwards.
- **The spread by the median absolute deviation** instead of the
  standard deviation: a far-off detection inflates the standard
  deviation, the very number meant to catch it, while the median
  absolute deviation (the middle value of the distances from the
  centre) does not move. Same procedure, same threshold.

## Per frame: pixels into the robot's frame

After calibration the matrix is applied to every frame as it comes, with
no averaging and no history (user, 2026-10-31): one matrix multiply per
marker.

**The multiply.** A homography works in *homogeneous coordinates*: a
pixel `(u, v)` is written as three numbers `(u, v, 1)`, multiplied by
`H`, and the three numbers that come out, `(X, Z, W)`, are divided by
the third to get back to two: `x = X / W`, `z = Z / W`. The division is
what makes it a perspective map rather than a plain stretch: `W` varies
across the image, so metres per pixel vary with it. This is what
`cv2.perspectiveTransform` does[^3].

```text
[X]       [u]
[Z] = H × [v]        x = X / W,  z = Z / W
[W]       [1]
```

With the camera exactly square to the rail, `W` is constant and the map
collapses to a scale and an offset: `x = s × (u − u₀)`,
`z = −s × (v − v₀)`, with `s` about 2.74 mm per pixel at 2.41 m[^1] and
`(u₀, v₀)` the pixel where the pole's plane's origin lands. The matrix
holds that case and every oblique one in the same nine numbers.

**Each frame:**

1. **Pivot** `(u_P, v_P)` through `H` gives `(x_P, z_P')`. Cart position
   is `x_P − x_C`: 0 at the centre, negative to the left. `z_P'` should
   be 0.082 m every frame; how far it strays is a running check on the
   detection and the matrix.
2. **End** `(u_E, v_E)` through `H` gives `(x_E, z_E)`.
3. **The pole's angle** from the arrow from Pivot to End, `Δx = x_E −
   x_P`, `Δz = z_E − z_P'`:

   ```text
   length = √(Δx² + Δz²)        should be 0.450 m
   cos θ  = Δz / length         1 straight up, 0 sideways, −1 hanging
   sin θ  = Δx / length         positive leaning towards +x
   θ      = atan2(Δx, Δz)       0 straight up, in radians
   ```

   The length is the second running check: a frame whose arrow is far
   from 0.450 m has a bad detection in it. `cos θ` and `sin θ` are what
   DeepMind's cart-pole observes (`trainer/dm_control_reward.md`), and
   `cos θ` is what its reward reads.
4. **Left and Right**, when seen, against their stored positions. A
   shift past a threshold (*Settings*) means the camera or the rig has
   moved and the matrix is stale: the run is refused until the
   calibration is run again. They never enter the position: they are in
   the other plane, and they never move.
5. **A missing slot** `(−1, −1)`: no cart position from a frame with no
   Pivot, no angle from one with no Pivot or no End. What the
   observation carries for that frame is not decided here.

Nothing here needs the frame before or after it. The velocities DeepMind
also observes are not in a single frame; if the policy is to have them,
they come from differences between the pool's timed entries, which this
plan does not design.

## In the synthetic loop

Live and virtual are one source (`AGENTS.md`, *The real robot*): the
synthetic Orin runs this same calibration against the player's frames,
with MuJoCo holding the cart at the three stops, and uses the same
per-frame step after. The stops' `x` values are then the MJCF's, the
same toml. How the orchestrator drives the cart to the stops, and how
the derived cart position and angle reach the policy, are changes to the
loop's design and land in `synthetic-loop-dia.md` first; they are not
designed here.

What the loop adds is the check: the trainer has MuJoCo's true Pivot and
End for every frame, for the reward alone. Comparing the matrix's cart
position and angle against them over a run measures this scheme's error
end to end, without that truth ever reaching the policy.

## What it does not cover

- **Lens distortion.** A homography is a straight-line map; a real lens
  bends straight lines, most at the frame's edges. If the residuals or
  the running checks show it, the chessboard calibration `ArUco.md`
  already lists undistorts each frame before the matrix is applied, and
  the matrix is fitted on undistorted pixels.
- **Depth.** The map is of one plane. Nothing in the base plane is
  measured, and `y` never is: the policy gets `(x, θ)`, as DeepMind's
  gets cart position and `cos θ`.
- **The detector's own error.** Under a pixel today (user, 2026-10-31):
  under 2.7 mm of cart position. The matrix removes the systematic error
  and leaves this one.
- **Pivot's and End's markers being hidden** by the pole or a hand:
  step 5 above, undecided.

## Settings

All in the Orin's toml (proposed: a `[calibration]` table in
`orin/settings.toml`, or a file it names):

```text
gap_left           stopped cart's left face to the left support wall, measured by hand (m)
gap_right          stopped cart's right face to the right support wall, measured by hand (m)
left_wall_x        10-Left-Rail.Site x, −0.485 (m)
right_wall_x       12-Right-Rail.Site x, 0.470 (m)
left_face_to_pivot    cart's left face to Pivot centre, 0.035 (m), from 11-Left-Cart.Site
right_face_to_pivot   cart's right face to Pivot centre, 0.035 (m), from 13-Right-Cart.Site
pivot_z            Pivot's height, 0.082 (m)
pole_length        Pivot to End, 0.450 (m)
frame              728 × 544, the detector's frame
min_detections     the fewest detections a stop's marker needs, 30
window_s           the pause at each stop, past the floor (s)
clip_k             spreads from the centre beyond which a detection is dropped (proposed 3)
residual_px        the largest residual a fit may have (proposed 1)
moved_px           the Left/Right shift that refuses a run
matrix_path        where the stored matrix lives on the Orin
```

## Open

- The two gaps: to be measured on the rig and entered.
- The missing-slot rule for the observation.
- The pool contract: cart position and angle with or instead of the
  pixel slots (a diagram change).

## References

[^1]: **2.7 mm per pixel.** The Pi GS camera's sensor is 4.968 mm wide
    behind a 6 mm lens (`AGENTS.md`, *Verified contracts*, the label
    projection), a horizontal field of 2 × atan(2.484 ÷ 6) = 45°. At the
    2.41 m `ArUco.md` gives for covering the 2000 mm stage, the frame
    spans 2 × 2.41 × tan(22.5°) = 1.997 m, and 1.997 m ÷ 728 px =
    2.74 mm per pixel. Worked out, not measured.

[^2]: **The depth error.** Pivot's plane is at `y` −0.132, Left's and
    Right's at −0.077 (the sites, above), 0.055 m apart. A plane 0.055 m
    nearer or further from a camera 2.41 m away is imaged 0.055 ÷ 2.41 =
    2.3% larger or smaller. Applied to the 0.44 m from the centre to an
    end, 10 mm. Worked out, not measured.

[^3]: **OpenCV's homography functions.** `cv2.findHomography(srcPoints,
    dstPoints, method=0)` finds the perspective transformation between
    two planes by least squares over all the points when `method` is 0
    (its RANSAC and LMEDS methods reject outliers, which clean
    calibration points do not need); `cv2.perspectiveTransform(src, m)`
    applies a 3×3 matrix to points with the homogeneous division
    ([OpenCV 4.x, *Camera Calibration and 3D Reconstruction*](https://docs.opencv.org/4.x/d9/d0c/group__calib3d.html),
    `findHomography`; [*Operations on arrays*](https://docs.opencv.org/4.x/d2/de8/group__core__array.html),
    `perspectiveTransform`; the pages refused an automated fetch on
    2026-10-31, so the descriptions are from memory of them, to be
    checked by hand).
    OpenCV is already a dependency of `orin/` (`orin/pyproject.toml`,
    `opencv-python-headless`), imported by `orin/src/detector.py`.
