# ArUco markers for the cart-pole

Codified research so the marker work doesn't depend on this chat's context
window. Everything in here was verified against OpenCV `4.x` source
(`modules/objdetect/src/aruco/`) unless marked otherwise.

## Requirements

- 4 unique, low-res, black-and-white markers, physical size slated at
  **50 x 50 mm**.
- Applied to: **left base**, **right base**, **pole pivot**, **pole end**.
- The **pole pivot marker doubles as the cart position**: the pole pivots on
  the cart, so detecting the pivot marker locates the cart.
- The two **base** markers (left, right) bound the available cart travel
  distance and provide redundancy: the pole can occlude one of them, but at
  least one base marker always remains visible.
- Physical dimensions: **track span 1000 mm** (left base to right base),
  **pole length 500 mm** (pole pivot to pole end).
- The pole pivot and pole end markers **rotate continuously** with the pole,
  so every marker must be unique through a full 360 deg.
- Used by: the Unity synthetic-dataset generator (auto-labeling) and the
  RPi 5 real-world detector.

## Chosen dictionary

- OpenCV pre-defined **`DICT_4X4_50`** (4x4 bits, 50 codes), IDs **0-3**.
- Rotation-invariant uniqueness is guaranteed by dictionary construction:
  the byte tables store **all 4 rotations** of every marker, and generation
  enforces a minimum Hamming distance between every rotation of every code.
  `DICT_4X4_50` corrects 1 bit error (`maxCorrectionBits = (4-1)/2`).
- Continuous rotation (not just 90 deg steps) is handled at detection time:
  the view is perspective-rectified to a flat grid and matched against the
  4 canonical rotations.
- Optional upgrade: `DICT_5X5_1000` or `DICT_6X6_250` IDs give fatter error
  margins (at 50 mm, 6x6 cells are ~8.3 mm). Uniqueness-through-rotation is
  guaranteed by any pre-defined dictionary.
- **Decision: 4x4 wins.** At the worst-case 36 px marker width (far stage
  edge, GS cam at 2.41 m), 4x4 gives ~9 px/cell vs ~4.5 px/cell for a 6x6
  tile with ring - and the rotating pole markers are the ones seen at the
  most extreme angles, where bigger cells tolerate motion blur better. The
  6x6's extra error correction is worthless when cells are unreadable. If a
  specific marker proves hard to read, make THAT marker physically larger
  instead of switching dictionaries.

## Dictionary data format (OpenCV 4.x)

- Bit tables live in
  `opencv/opencv -> modules/objdetect/src/aruco/predefined_dictionaries.hpp`:
  `DICT_4X4_1000_BYTES[][4][2]` - per marker, 4 rotations x 2 bytes (16 bits).
- **`DICT_4X4_50` = the first 50 entries of the 4X4_1000 table** (verified in
  `aruco_dictionary.cpp`: `Mat(50, 2, CV_8UC4, (uchar*)DICT_4X4_1000_BYTES)`).
- Bit expansion, `Dictionary::getBitsFromByteList`
  (`modules/objdetect/src/aruco/aruco_dictionary.cpp`):
  - rotation `r` reads the 2 bytes at `byteOffset = r * 2` (the 4 rotations
    are pre-baked in the table - a generator does not rotate anything),
  - bits are expanded **MSB first**, filling the 4x4 grid **row-major**.
- Rendering, `Dictionary::generateImageMarker(id, sidePixels, borderBits)`:
  - canvas of `(markerSize + 2 * borderBits)^2` cells, initialized to 0,
  - the `borderBits` (default 1) wide frame therefore renders **always
    black** - the guaranteed black ring is NOT part of the codeword,
  - the inner 4x4 cells are the dictionary bits with
    **bit 1 = white (255), bit 0 = black (0)** - the inversion gotcha,
  - nearest-neighbor resize to the requested side pixel count.

## Gotchas (found the hard way)

1. **Bit 1 = white**, not black. Assuming 1 = black makes the corner/border
   checks fail for no visible reason.
2. The guaranteed black ring comes from `borderBits`, not from the codeword.
   Mentally rendering the marker as a 6x6 tile (black ring + 4x4 grid) is a
   valid and useful model.
3. The C array is `[marker][4][2]` while OpenCV reads it as a
   `Mat(rows, 2, CV_8UC4)` - the channel/interleave mapping means rotation 0
   is simply the first byte pair in file order, but pixel-exact verification
   against `cv::aruco.generateImageMarker` is still recommended before
   trusting any hand-decoded grid.

## Hand-extracted grids (DICT_4X4_50, rotation 0, `1` = white)

Decoded from `predefined_dictionaries.hpp`. **Caveat**: decoded by hand from
the byte format above; verify against
`cv::aruco.generateImageMarker(DICT_4X4_50, id, N)` before pixel-critical use.

| ID | rotation-0 bytes | grid rows (MSB-first, row-major) |
| --- | --- | --- |
| 0 | 181, 50 | `1011` / `0101` / `0011` / `0010` |
| 1 | 15, 154 | `0000` / `1111` / `1001` / `1010` |
| 2 | 51, 45 | `0011` / `0011` / `0010` / `1101` |
| 3 | 153, 70 | `1001` / `1001` / `0100` / `0110` |

## Rust generator spec (planned)

- Input: dictionary id range (DICT_4X4_50, ids 0-3), output side pixels.
- Read the 2 rotation-0 bytes per marker from the canonical table, expand
  MSB-first row-major into the 4x4 grid.
- Render a 6x6 tile: 1-cell black ring, inner cells = grid with
  **bit 1 -> white**.
- Nearest-neighbor scale to the output resolution (e.g. 1024 px).
- Output PNGs as `marker_<id>.png`.
- Verify: render ID 0..3 and diff against OpenCV `generateImageMarker`
  output for the same parameters (do this once when OpenCV is available).

## Physical sizing (50 x 50 mm)

`generateImageMarker` draws the grid plus a 1-cell black ring, so the drawn
tile is 6x6 cells for a 4x4 grid:

- If **50 mm = the full printed tile** (ring included): each cell is
  8.33 mm, the coded grid is 33.3 mm.
- If **50 mm = the coded grid**: the tile is 75 mm total (ring is 12.5 mm on
  each side).

Decision pending: which of the two "50 x 50 mm" means. The placement tool
should draw an additional white quiet zone (>= 1 cell) outside the black
ring so the marker is separable from the part surface.

## Pipeline context

- Marker anchors are tiny (1 mm) cylinder parts placed in the FreeCAD
  assemblies at the four marker spots and linked into the top assembly.
  Shapeless empties do not survive the STEP export - the anchors carry
  geometry so they export and appear as Unity nodes.
- The Unity `MarkerManager` (on the scene root) finds each anchor by node
  name, parents a 50 mm marker quad to it (position/orientation from the
  anchor placement) and **disables the anchor's renderer and colliders** so
  the 1 mm cylinder plip never enters renders or physics.
- Entry order in the MarkerManager defines the YOLO class id:
  0 = left base, 1 = right base, 2 = pole pivot, 3 = pole end.
- Markers are rendered into the synthetic dataset; ground truth labels come
  from projecting the marker quad corners through the camera.
- The trained detector (YOLOv8n / YOLO11n class, ONNX) runs on the RPi 5.
- The real rig carries the printed 50 mm markers, so the same
  `DICT_4X4_50` decoding works for both synthetic and real images.

## Resolution budget (stage = 2000 x 1000 mm)

The real stage the camera must cover is 2000 mm wide by 1000 mm high.

- 1080p over 2000 mm = **0.96 px/mm** -> a 50 mm marker at the far stage
  edge is ~48 px wide = ~8 px/cell -> ArUco decode robust everywhere on the
  stage.
- Detector input at 480 px = 0.24 px/mm -> marker ~12 px (2 px/cell):
  enough to *detect* the chunky blob, too coarse to *decode*.
- Therefore the runtime is two-stage: YOLO letterboxed at 480 px finds the
  markers; each detection is decoded with ArUco on a crop of the full-res
  1080p frame (~48 px markers).
- 16:9 at 2000 mm wide covers 1125 mm of height, so the 1000 mm stage fits
  with margin. Camera distance for a lens of HFOV theta:
  `distance = 1000 mm / tan(theta / 2)` (~1.7 m at 60 deg).
- Training images mirror this: render the synthetic frames at the same
  aspect and rough scale so the detector sees markers at realistic sizes.
- Capture stays color (dataset converts to grayscale per deployment if
  needed); the Pi capture pipeline already does 30 fps at 1080p.

## Camera: Raspberry Pi Global Shutter Camera (IMX296LQR-C)

Verified from the official product brief (`RP-008196-DS-gs-camera-product-brief.pdf`):

- Sensor: Sony **IMX296LQR-C** - **color**, global shutter, 1.58 MP
  (1440 x 1080 effective), 3.45 um square pixels, 6.3 mm sensor diagonal.
- **CS-Mount** lens standard (same lens family as the HQ Camera) - the FOV
  depends on the mounted lens.
- **Mounted lens: Raspberry Pi 6 mm Wide Angle (SC0124 / PT361060M3MP12)**,
  F1.2, image format 1/2", MOD 0.2 m, spec field angle 63 deg (diagonal,
  1/2" format). On the IMX296 active area (4.968 x 3.726 mm) it delivers:
  - HFOV ~45 deg, VFOV ~35 deg
  - distance to cover the 2000 mm stage: **~2.41 m**
  - F1.2 is fast enough for indoor desk lighting; MOD 0.2 m is no
    constraint at that distance.
- One-time setup task: chessboard calibration + undistortion before using
  marker pose from pixels - wide-angle M12 lenses carry barrel distortion,
  which the detector tolerates but marker pose accuracy does not.
- Resolution budget on this sensor (1440 px over 2000 mm stage = 0.72
  px/mm): 50 mm marker = **36 px** = 6 px/cell at the far stage edge -
  decode still robust; detector input at 480 px sees ~12 px (detect-only).
- Global shutter is the key spec here: the pole marker swings fast, and a
  rolling shutter would shear the marker quad during motion - the GS camera
  captures the whole frame at once.
- The sensor is **color**, so the detector trains on color synthetics
  directly (no grayscale conversion needed - deployment matches training).
