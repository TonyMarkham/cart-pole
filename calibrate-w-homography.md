# Calibrate with a homography — cart position and pole angle from the camera

The robot has to know where its cart is, and it has no sensor that says
so: the stepper reports nothing, and the camera sees only four marker
centres in pixels. This plan turns those pixels into metres along the
rail, and the pole's angle, with one matrix built once during the rig's
existing calibration and applied to every frame after.

Status: **plan** (user, 2026-10-31). The calibration itself is built, in
`__live__/` (2026-11-01): the moves, the measurement, the fit and the stored
matrix. It has not yet produced a matrix from the real rig, and nothing uses a matrix
yet (*Per frame*). It is a new
scheme, on the `feature/dm_control` branch: the policy reads the cart's position and
the pole's angle, as DeepMind's cart-pole does
(`__synthetic__/trainer/python/dm_control_reward.md`, *Where the reward's numbers come from*),
derived on the Orin from the detections. The synthetic loop has its own calibration
(*In the synthetic loop*), and shares no code with this one.

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
aside (*The lens*). The pole's plane is such a plane:
Pivot and End always lie in it, whatever the cart and pole do, because
the cart runs along a straight rail and the pole swings in one flat
plane.
One matrix maps that plane; fitting it needs four or more points in the
plane whose positions are known in both pixels and metres, no three on
a line.

## Terms

- **The robot's frame of reference:** the MJCF's world frame, in which
  every site is given (`__synthetic__/trainer/python/reward-plan.md`, *Where the reward's
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
  hanging (`__synthetic__/trainer/python/reward-plan.md`, *Where the reward's numbers come
  from*): Pivot `(−0.010, −0.132, 0.082)`, End `(−0.010, −0.132,
  −0.368)`, 0.450 m straight below it; Left `(−0.510, −0.077, 0.025)`,
  Right `(0.495, −0.077, 0.025)`. Pivot's height, `z_P` = 0.082 m, never
  changes; End hanging is always at `z_P − 0.450`.
- **The three stops:** the cart stands on one limit switch, then on the
  other, then exactly at the centre, halfway between the two trip points.
  Where the switches trip is the one thing the CAD knows only in theory:
  the physical build varies, so it is measured on the rig (user,
  2026-10-31), with the two gaps and the span below.

**The stops' `x`, from real measurements, which win over the CAD** (user,
2026-11-01). Two kinds are made on the rig. With the cart stopped on each
switch, a human measures the gap between the cart's face and the wall of the
rail support it faces, about 5 mm:

- `gap_idler`: at the idler's end, on the left, the cart's face to that support's wall.
- `gap_motor`: at the motor's end, on the right, the cart's face to that support's wall.

The human types these into the page at each switch (user, 2026-11-01), in
millimetres, and they are required: they account for the difference between the
CAD and the real build.
The board measures the other, the **span**: its count of pulses from one switch
to the other. The rest comes from the MJCF, each value citing its site:

```text
left support wall, x               10-Left-Rail.Site                                  -0.485
right support wall, x              12-Right-Rail.Site                                  0.470
cart's left face to Pivot centre   Pivot x - 11-Left-Cart.Site x = -0.010 - (-0.045)   0.035
cart's right face to Pivot centre  13-Right-Cart.Site x - Pivot x = 0.025 - (-0.010)   0.035
```

The gaps and the CAD say where the rail's centre is. The span says how far apart
the stops are:

```text
x_left_cad  = left_wall_x  + gap_idler + left_face_to_pivot
x_right_cad = right_wall_x - gap_motor - right_face_to_pivot
x_centre    = (x_left_cad + x_right_cad) / 2

x_idler     = x_centre - (span_pulses x metres_per_pulse) / 2
x_motor     = x_centre + (span_pulses x metres_per_pulse) / 2
```

`metres_per_pulse` is 0.015 mm: `__synthetic__/trainer/python/settings.toml` `[drive]`, a 96,000 um
belt turn over 200 full steps x 32 microsteps.

**Why the span and not the CAD's length.** On 2026-11-01 the board measured
59,822 pulses, which is 897.3 mm, repeatably to one pulse. The CAD allows at most
885 mm, even with both gaps at zero, and 873 mm with gaps of 5 and 7 mm. The real
measurement wins, and the difference is reported with each fit, never used. What
causes it is not known.

Left and right are as the camera sees them. The idler's end is on the left and the
motor's on the right (measured on the rig, 2026-11-01: Pivot at 175 and 536 pixels from
the picture's left edge).

## The calibration sequence

The camera calibration rides on the board's moves: the cart's three stops are
the moments its position is known, and the Orin measures at each (user,
2026-10-31). **A person runs the stages from the page** (user, 2026-11-01), and
steadies the pole at each stop, so the pole is still when it is measured. The
person presses every button and watches, so nothing needs to trigger the moves
automatically. No stop ends before it has been measured or the person has chosen
to leave it.

1. **Stop A and stop B, the two switches.** The person sends the cart to one
   switch, then to the other. Each seek is slow, and the board reports the trip.
   The second one's pulse count is the span.
2. **Stop C, the centre.** The person sends the cart to the centre, half the span
   from the switch it rests on.
3. **At each stop, the measurement** (*The measurement at a stop*). Two known
   points: Pivot `(x, z_P)`, End `(x, z_P - 0.450)`.
4. **Fit.** After the third stop has been kept, six point pairs, each a pixel
   position and a metre position in the pole's plane:

   ```text
   pair         pixels (u, v)            metres (x, z)
   Pivot motor  clipped mean at the stop (x_motor,  0.082)
   Pivot idler  clipped mean at the stop (x_idler,  0.082)
   Pivot centre clipped mean at the stop (x_centre, 0.082)
   End motor    clipped mean at the stop (x_motor,  -0.368)
   End idler    clipped mean at the stop (x_idler,  -0.368)
   End centre   clipped mean at the stop (x_centre, -0.368)
   ```

   The three Pivots lie on one line, the rail; the three Ends lie on a line below
   it. A fit needs four points with no three on a line, and has them in any two
   Pivots and any two Ends; the other two points overdetermine it, which is what
   makes the residuals a check (with exactly four points the fit is exact and every
   residual is 0).

   The pixels are first corrected for the lens, with the number that leaves the fit
   closest to all six points (*The lens*). `cv2.findHomography(corrected, metres,
   method=0)` fits `H`, corrected pixels to metres, by least squares[^3]. Then each
   metre point is mapped back to pixels through the inverse and compared with its
   corrected pixel position: the residuals. Every residual under the threshold
   (*Settings*; 1 px) accepts the fit; one over it fails the calibration, nothing is
   stored, and the stops are measured again.
5. **Store.** The lens number, the nine numbers of `H`, the residuals, the six pairs, every marker's
   point at each stop with its count, spread and number dropped, Left's and Right's
   among them, the span and the CAD's length, the frame size the pixels refer to
   (728x544), and the date, in one file on the Orin at the path the settings name.

### The lens

The lens is a Raspberry Pi 6 mm wide angle (`ArUco.md`, *Camera*), and it bows straight
lines outward from the middle of the picture: barrel distortion. On 2026-11-01 this was
the whole of a failed fit. With no correction the worst point was 1.600 px off, and three
things showed the bend: Pivot's row bowed 0.86 px upward, End's row 2.41 px downward, each
away from the picture's middle, and the pole looked longer at the centre (184.4 px) than
at the ends (180.6 and 181.6 px). A homography maps straight lines to straight lines, so
no number of images can fix that.

One number describes the bend, `k`. Each point moves along the line through the
picture's centre, further the further out it is:

```text
r2        = x^2 + y^2        x, y: the point's distance from the centre, in focal lengths
corrected = centre + (point - centre) x (1 + k x r2)
```

**It needs no chessboard** (user, 2026-11-01): the six points already have known real
positions, three columns the measured span apart and two rows exactly the pole's length
apart, which makes them a small measured grid. So each fit searches for the `k` that
leaves the matrix closest to all six, and stores it with the matrix. On the rig `k` was
+0.44, the worst point fell to 0.166 px, and every point landed within about 0.4 mm of
where the pulses, the pole and the gaps put it. Separately, the `k` that merely
straightens each row, using no distances at all, was +0.34 and +0.48.

**What that does and does not show.** The fit chooses 9 numbers (the matrix's 8 and `k`)
from 12 measurements, so only 3 are left to check it, and the six points look good partly
because they were fitted. The picture's centre stands in for the lens's centre, which is
not measured. The points cover only the rail and the hanging pole: End upright sits near
the top of the picture, outside them, where the bend is greatest. A chessboard
(`ArUco.md`) would measure the whole picture, if that proves necessary. Checking the
matrix at a place it was not fitted to, such as a known pulse count from a switch, is the
independent test, and has not been done.

### The measurement at a stop

Turns a stop's many detections of a still marker into one point, trusting no
single detection (user, 2026-10-31). The pole hardly moves at these speeds, so a
person stops what movement there is before the measurement starts (user,
2026-11-01), and End is then measured the same way as Pivot. Checking that the pole
really is still is left for later: the first version assumes it (user, 2026-11-01).

In a window over the page (user, 2026-11-01), so nothing behind it can be pressed:

1. The cart arrives. At each end, the window asks for the gap (*The stops' `x`*),
   first, because measuring it could set the pole swinging. It asks the person to
   steady the pole.
2. The person presses NEXT.
3. A box asks how many images to use, 30 by default, from 30 to 9,999. An image
   counts only if it has both Pivot and End in it. The person presses START.
4. A progress bar and text, with a slide show of the frames, Pivot's box in green
   and End's in blue. The show samples the frames, so it costs the measurement
   nothing.
5. A summary. Done keeps the stop's result and closes the window, ready for the next
   move. Retry throws it away and starts again from the first step.

Cancel is on every screen, and leaves the cart where it is. A button on the page
opens the window again for the stop the cart is at. The cart cannot move during a
measurement, and a measurement cannot start while the cart is moving.

**More images give a more accurate point,** because the noise in a point made from
`n` detections shrinks as `1 / sqrt(n)`.

**Each point is a clipped mean, not a plain one.** The marker is still, so its
detections scatter about one place, but the detector can return a wrong box now and
then, and one detection 20 px off pulls a plain mean of 30 by 0.7 px, more than the
detector's own error. So: take the centre of the detections (their median, the
middle value, which a far-off point cannot drag), their spread about it (the root
mean square of their distances from it), drop every detection further from the
centre than `clip_k` spreads (proposed 3), and take the plain mean of the rest. The
count, the spread and the number dropped are stored with the point.

**Measured on the rig, 2026-11-01,** with the cart still and nothing touching it:
100 images in 2.1 s, about 47 frames a second. Pivot's spread 0.025 px, End's 0.031
px, and 0 and 1 of the 100 dropped. Two runs, of 60 and 100 images, agreed to within
0.007 px. That is how repeatable a still marker's point is, not how accurate it is:
accuracy needs the fit's residuals, from a real calibration.

**The frames.** The sensor's full 1456x1088, halved exactly to 728x544 in greyscale
by the camera software, then encoded as a JPEG at quality 90 for the detector: the
format of the 146 real images in `datasets/real-640`, on which the model has been run
(the model itself was trained on the synthetic 1456x1088 images, `datasets/generated`,
which YOLO scaled down).

Follows from rules already set:

- **The metre values are settings, not code** (`AGENTS.md`, *Tunable values live in
  a toml*): the walls, the faces, `z_P`, the pole's 0.450 m and the metres per pulse
  all come from the toml, so a rebuilt rig changes the file, not the program.
- **The image limits, `clip_k` and the residual threshold are settings** too.

Proposed, not decided:

- **Keeping every frame's detections** with the stored matrix, so a bad fit can be
  read afterwards. Today only each stop's result is kept.
- **The page's drawing and `control-design.md`:** the drawing now shows the idler on the
  left, as the camera sees it. `control-design.md` calls D2 left and D3 right, which is
  not the camera's view, and has not been changed.

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

**Each frame:** every marker's pixels are first corrected for the lens with the stored
`k` (*The lens*), because `H` is fitted on corrected pixels.

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
   DeepMind's cart-pole observes (`__synthetic__/trainer/python/dm_control_reward.md`), and
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

Built, and kept apart from the rig's code (user, 2026-11-01). `synthetic-loop-dia.md`,
*Calibration*, is its design. It follows this plan's arithmetic, with three differences:

- **The metres are MuJoCo's.** Each point's true position is read from its site, so the
  hand-measured gaps, the span and the toml's wall positions have no part in it.
- **Seven poses, not three** (user, 2026-11-01): the pole hanging at the three stops, as
  on the rig, and also horizontal and straight up at the end stops. MuJoCo can hold a
  pole anywhere, and End swings well outside the area the hanging pole covers.
- **The policy gets points, not an angle** (user, 2026-11-01). Pivot's and End's `(x, z)`
  go in the pool, in metres. Steps 1 to 3 of *Per frame* apply as far as the two points;
  the angle and the running checks are not built.

**Measured on 2026-11-01**, through the real player and detector. The fit's worst point
was 0.103 px (0.112 px with no lens number), and the lens number was −0.005. The loop's
scene has no lens distortion, so about 0 is what it should be. At 13 poses the fit never
saw, such as the pole 45° up and leaning out from each end stop, the mapped points were
at most 0.76 mm from the rig's true geometry in any single frame, about 0.3 of a pixel.
Over one 221-frame episode of random commands, the mean error was 0.23 mm for Pivot and
0.21 mm for End. These describe a camera with no lens distortion, so they say nothing
about the rig's.

## What it does not cover

- **Lens distortion beyond one number,** or outside the area the six points cover
  (*The lens*).
- **Depth.** The map is of one plane. Nothing in the base plane is
  measured, and `y` never is: the policy gets `(x, θ)`, as DeepMind's
  gets cart position and `cos θ`.
- **The detector's own error.** Under a pixel today (user, 2026-10-31):
  under 2.7 mm of cart position. The matrix removes the systematic error
  and leaves this one.
- **Pivot's and End's markers being hidden** by the pole or a hand:
  step 5 above, undecided. During a measurement a hidden marker just stops the count,
  and the person presses Cancel.
- **A pole that is still moving** during a measurement: assumed not to be, for now
  (*The measurement at a stop*).

## Settings

The two gaps are typed into the page at each switch, and kept with the fit. The rest
are all in `__live__/orin/python/settings.toml`, `[calibration]`:

```text
max_gap_mm         the largest gap the page accepts, 50 (mm), to catch a mistyped number
left_wall_x        10-Left-Rail.Site x, -0.485 (m)
right_wall_x       12-Right-Rail.Site x, 0.470 (m)
left_face_to_pivot    cart's left face to Pivot centre, 0.035 (m), from 11-Left-Cart.Site
right_face_to_pivot   cart's right face to Pivot centre, 0.035 (m), from 13-Right-Cart.Site
pivot_z            Pivot's height, 0.082 (m)
pole_length        Pivot to End, 0.450 (m)
metres_per_pulse   cart travel per pulse, 0.000015 (m), from __synthetic__/trainer/python/settings.toml [drive]
min_images         the fewest images a stop may be asked for, 30
max_images         the most images a stop may be asked for, 9999
clip_k             spreads from the centre beyond which a detection is dropped (proposed 3)
residual_px        the largest residual a fit may have, 1 (px): the rig's was 1.600 with no lens correction, 0.166 with one
lens_focal_mm      the lens's focal length, 6.0 (mm)
sensor_pixel_um    the sensor's pixel, 3.45 (um); halving the frame makes it 6.9
lens_k_min, lens_k_max, lens_k_step   the range and step searched for the lens number
matrix_path        where the stored matrix lives on the Orin
kept_path          where each stop's kept result is saved, so a restart does not lose it
```

Not yet: `moved_px`, the Left/Right shift that refuses a run, belongs with the
per-frame step, which is not built.

## Open

- Why the measured span is at least 12 mm longer than the CAD allows.
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
    OpenCV is a dependency of `__live__/orin/python`
    (`opencv-python-headless`), imported by its `src/detector.py`.

## Appendix: implementation

### Where it lives in the repo

The calibration runs on three computers, in two languages, so it has no
single home: each part sits under the computer it runs on (user,
2026-10-31). `__live__/` holds what runs on the real rig and
`__synthetic__/` is its counterpart for the synthetic loop. Each computer
has a folder per language, and only `__synthetic__/orin/python/` has two.

```text
cart-pole/
├── Cargo.toml                       the root workspace, which builds the [root] crates
├── calibrate-w-homography.md        this plan
├── __synthetic__/                   the synthetic loop's own calibration (its README)
└── __live__/
    ├── arduino/
    │   └── rust/
    │       └── crates/
    │           └── calibration/     [root] the firmware: the cart's three stops
    ├── orin/
    │   ├── python/                  a uv project: measures the stops, fits the matrix
    │   │   ├── pyproject.toml
    │   │   ├── uv.lock
    │   │   ├── settings.toml
    │   │   └── src/
    │   │       └── capture_push.py  one picture, pushed to the browser
    │   ├── rust/                    the Orin's workspace, pushed and built there
    │   │   ├── Cargo.toml
    │   │   ├── pages/               the built browser pages, from wasm/
    │   │   └── crates/
    │   │       ├── image-push/      the command-line tool that sends a picture
    │   │       └── webserver/       receives pictures, serves the pages
    │   └── wasm/
    │       ├── arduino/             [root] the Arduino's pins, in the browser
    │       └── viewer/              [root] the camera's pictures, in the browser
    └── rpi/
        └── rust/                    the Pi's workspace, pushed and built there
            ├── Cargo.toml
            └── crates/
                └── arduino-manager/ flashes the Arduino and logs its serial line
```

A **crate** is one Rust package. A **workspace** is a group of crates built
together from one `Cargo.toml`. A **uv project** is a Python package whose
dependencies `uv` installs into its own `.venv` folder, leaving the
computer's Python alone.

### What each part does in the calibration

```text
part                             runs on   its job (The calibration sequence)
arduino/rust                     Arduino   makes each move on request, stops, reports which pin tripped
orin/python                      Orin      captures, detects, measures each stop, fits and checks H, stores it
orin/rust                        Orin      serves the page, sends the moves, forwards the measuring to the Python
orin/wasm/calibrate              a browser the page a person runs the calibration from
rpi/rust                         Pi        flashes the firmware and logs the serial line, before the run
```

- **The Python on the Orin holds the arithmetic,** because detection on the
  robot is Python (`AGENTS.md`, *Python policy*) and the fit is OpenCV's
  `cv2.findHomography`[^3].
- **The manager takes no part in the sequence.** Its link is the USB cable.
  With the cable in, both limit switches have read open, though not every time
  (`__live__/rpi/rust/crates/arduino-manager/README.md`). The moves reach the board
  over WiFi, as UDP datagrams from the Orin's webserver.

### What exists today

```text
part                             today (2026-11-01)
arduino/rust                     finds both ends and the centre on request, hears Stop mid-move. On the rig's board.
orin/python                      camera, detector, measurement, fit and store, as a service. On the Orin in ~/live-orin-python.
orin/rust                        routes for the moves and the measurement, forwarded to the service. On the Orin in ~/orin-cam.
orin/wasm/calibrate              the calibration page, with the measurement's window.
rpi/rust                         on the Pi: flashes the board and logs it.
__synthetic__                    its own calibration, built and measured (In the synthetic loop).
```

The older folders stay where they are: `calibrate/` and `bench-station/` measure the
motor's limits, and `__synthetic__/orin/python/` is the synthetic Orin, whose detector `__live__/orin/python`
copied on 2026-11-01 (user: a copy for now, the obsolete code to be cleaned up later).

### How each part builds

`__live__` is kept apart from the repo (user, 2026-10-31). What is built on the
Pi or the Orin is that computer's own workspace, pushed whole and needing
nothing else from the repo. Only the firmware and the browser pages are built
on the PC, as members of the root workspace, which needs the `unor4-hal`
checkout beside the repo.

```text
part                       built on   from                 with
arduino/rust/crates/calibration  PC   its own folder       cargo build --profile firmware
orin/wasm/*                the PC     its own folder       trunk build --release
orin/rust                  the Orin   its workspace root   cargo build --release -p webserver -p image-push
rpi/rust                   the Pi     its workspace root   cargo build --release
orin/python                the Orin   its own folder       uv run python src/<script>.py
```

Each part's README has the full steps; the Python's usage is at the head
of its script.

### Changes the layout brings

Proposed, not decided:

- **The stored matrix stays out of git.** `matrix_path` names a file on
  the Orin, outside the repo, because a matrix belongs to one camera
  position on one rig. The two gaps, also that rig's, are in the settings
  file by the plan's rule and so are committed.
- **`image_push` in the settings stays at `~/orin-cam`:** the copy is
  deployed into that folder on the Orin (user, 2026-10-31).

Requirements on the Orin, from what it has (checked on `robo-orin`,
2026-10-31):

- **The repo has to be on it, on this branch.** `~/git/cart-pole` is on
  `feature/sim-loop` with no `__live__/`, which is why the capture ran
  from a temporary copy.
- **The `.venv` must see the system's Python packages,** because TensorRT
  (the NVIDIA runtime the detector uses) is installed only for the system
  Python 3.10, in `/usr/lib/python3.10/dist-packages`.
  `~/orin-test/.venv` is set up this way.

### Open

- **What `orin/rust` adds to orin-cam** beyond showing pictures.
