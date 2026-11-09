"""The lens measurement's arithmetic: where the box goes, the board's shape inside it,
whether the board is held there, and OpenCV's calibrateCamera on the kept stills.

The lens bends the picture, most at the edges, and the matrix that turns pixels into
metres cannot follow a bend. calibrateCamera measures the bend from many stills of a
board whose layout is known: the lens's centre, its focal lengths and the bend's
coefficients (OpenCV, "Camera Calibration").

The board's size in the picture comes from the camera's scale measured from the
board itself, so nothing from the rig's earlier measurements goes in. The board's
32.25 mm markers measured 18.5 px at 2 m, and the scale grows in proportion as the
board comes closer:

    mm per pixel = 32.25 mm / 18.5 px x 1000 mm / 2000 mm = 0.872 mm at 1 m
    board        = 6 x 43 = 258 mm by 4 x 43 = 172 mm  ->  296 x 197 pixels
    box          = the board plus box_margin, 20%       ->  355 x 237 pixels

The box is larger than the board so a hand-held board fits inside it with room.
The flat positions tile the whole picture with the box, row by row from the top
left, the first and last of each row and column touching the picture's edges. The
tilted positions follow: the board turned so one side is farther from the camera.
Stills of a board held flat to the camera, wherever it is, all tell calibrateCamera
the same thing about the lens; Zhang's method, which calibrateCamera is built on,
needs the board at different angles ("A Flexible New Technique for Camera
Calibration", 2000: if the board "undergoes pure translation, then the second image
does not provide additional constraints").
"""

import math
from dataclasses import dataclass

import cv2
import numpy as np

# The sides a board can be turned away from the camera, as the picture shows them.
TILTS = ("left", "right", "top", "bottom")

# Where a tilted position's box sits: the picture's centre or one of its corners.
SPOTS = ("centre", "top left", "top right", "bottom left", "bottom right")


@dataclass(frozen=True)
class Rect:
    x: int
    y: int
    w: int
    h: int

    def holds(self, points: np.ndarray) -> bool:
        """Whether every point `(u, v)`, one per row, is inside."""
        u, v = points[:, 0], points[:, 1]
        return bool(((u >= self.x) & (u <= self.x + self.w) & (v >= self.y) & (v <= self.y + self.h)).all())

    def to_json(self) -> dict:
        return {"x": self.x, "y": self.y, "w": self.w, "h": self.h}


@dataclass(frozen=True)
class Position:
    """One place to hold the board: the box every corner must be inside, the side
    turned away from the camera ("" when flat), and the shape the board makes
    there, drawn for the person to match."""

    rect: Rect
    tilt: str
    shape: list[list[float]]

    def to_json(self) -> dict:
        return {**self.rect.to_json(), "tilt": self.tilt, "shape": self.shape}


def mm_per_px(lens: dict, board: dict) -> float:
    """The camera's scale at the distance the board is held, from the board's own
    markers as measured."""
    marker_mm = board["printed_square_mm"] * board["marker_mm"] / board["square_mm"]
    return marker_mm / lens["measured_marker_px"] * lens["distance_mm"] / lens["measured_at_mm"]


def board_mm(board: dict) -> tuple[float, float]:
    """The board's printed squares, across and down, mm."""
    return board["squares_x"] * board["printed_square_mm"], board["squares_y"] * board["printed_square_mm"]


def rect_size(lens: dict, board: dict) -> tuple[int, int]:
    """The box in full-sensor pixels: the board's printed squares at the distance it
    is held, plus `box_margin`."""
    across, down = board_mm(board)
    scale = (1 + lens["box_margin"]) / mm_per_px(lens, board)
    return round(across * scale), round(down * scale)


def positions(width: int, height: int, size: tuple[int, int]) -> list[Rect]:
    """Boxes tiling a `width` x `height` picture, row by row from the top left."""
    w, h = size
    cols, rows = math.ceil(width / w), math.ceil(height / h)

    def spread(count: int, span: int) -> list[int]:
        return [0] if count == 1 else [round(i * span / (count - 1)) for i in range(count)]

    return [Rect(x, y, w, h) for y in spread(rows, height - h) for x in spread(cols, width - w)]


def spot(width: int, height: int, size: tuple[int, int], name: str) -> Rect:
    """The box at the picture's centre or at one of its corners."""
    w, h = size
    x = {"left": 0, "right": width - w}.get(name.split()[-1], round((width - w) / 2))
    y = {"top": 0, "bottom": height - h}.get(name.split()[0], round((height - h) / 2))
    return Rect(x, y, w, h)


def focal_px(lens: dict, board: dict) -> float:
    """The camera's scale as a focal length in pixels: a pinhole camera draws a
    length L at distance Z as f x L / Z pixels, and the board measured 1 / 0.872
    pixels per mm at 1 m, so f = 1000 / 0.872 = 1147 px. From the board's own
    measurement, not the lens's specs; good enough to tell a turn of 20 degrees
    from one of 30, not to calibrate with."""
    return lens["distance_mm"] / mm_per_px(lens, board)


def shape(lens: dict, board: dict, rect: Rect, tilt: str) -> list[list[float]]:
    """The board as a pinhole camera sees it when its centre is `distance_mm` away,
    turned `tilt_deg` so its `tilt` side is farther ("" for flat), centred in
    `rect`. Corners in order top left, top right, bottom right, bottom left."""
    across, down = board_mm(board)
    distance, scale = lens["distance_mm"], 1 / mm_per_px(lens, board)
    angle = math.radians(lens["tilt_deg"]) if tilt else 0.0
    points = []
    for x, y in ((-across / 2, -down / 2), (across / 2, -down / 2), (across / 2, down / 2), (-across / 2, down / 2)):
        # Turning about the board's centre: the turned axis shortens by cos, and the
        # named side moves away by sin. Left is -x, and the picture's v grows
        # downward, so top is -y.
        depth = distance
        if tilt in ("left", "right"):
            depth -= (x if tilt == "left" else -x) * math.sin(angle)
            x *= math.cos(angle)
        elif tilt in ("top", "bottom"):
            depth -= (y if tilt == "top" else -y) * math.sin(angle)
            y *= math.cos(angle)
        points.append((x * distance / depth * scale, y * distance / depth * scale))
    us, vs = [u for u, _ in points], [v for _, v in points]
    du = rect.x + rect.w / 2 - (min(us) + max(us)) / 2
    dv = rect.y + rect.h / 2 - (min(vs) + max(vs)) / 2
    return [[round(u + du, 1), round(v + dv, 1)] for u, v in points]


def plan(lens: dict, board: dict, width: int, height: int) -> list[Position]:
    """Every position in order: the flat ones tiling the picture, then each of
    `tilts` at each of `tilt_spots`."""
    size = rect_size(lens, board)
    flat = [Position(rect, "", shape(lens, board, rect, "")) for rect in positions(width, height, size)]
    tilted = []
    for name in lens["tilt_spots"]:
        if name not in SPOTS:
            raise ValueError(f"[lens] tilt_spots: {name!r} is not one of {', '.join(SPOTS)}")
        rect = spot(width, height, size, name)
        for tilt in lens["tilts"]:
            if tilt not in TILTS:
                raise ValueError(f"[lens] tilts: {tilt!r} is not one of {', '.join(TILTS)}")
            tilted.append(Position(rect, tilt, shape(lens, board, rect, tilt)))
    return flat + tilted


def outline(board: cv2.aruco.CharucoBoard, corners: np.ndarray, ids: np.ndarray) -> np.ndarray | None:
    """Where the board's outer corners are in the picture, from the ChArUco corners
    found: the matrix from the board's flat layout to the picture, fitted to them,
    applied to the board's four outer corners. None with fewer than 4 found, the
    least a matrix needs."""
    if ids is None or len(ids) < 4:
        return None
    layout = board.getChessboardCorners()[ids.ravel()][:, :2]
    matrix, _ = cv2.findHomography(layout.astype(np.float64), corners.reshape(-1, 2).astype(np.float64))
    if matrix is None:
        return None
    (squares_x, squares_y), side = board.getChessboardSize(), board.getSquareLength()
    edge = np.array([[[0, 0], [squares_x * side, 0], [squares_x * side, squares_y * side], [0, squares_y * side]]])
    return cv2.perspectiveTransform(edge.astype(np.float64), matrix).reshape(4, 2)


def turn(board: cv2.aruco.CharucoBoard, corners: np.ndarray, ids: np.ndarray, focal: float, width: int, height: int) -> dict | None:
    """How far each side of the board, as the picture shows it, is turned away from
    the camera, degrees; negative when it is turned towards it. OpenCV's solvePnP
    finds the board's pose from the corners found, with a camera of focal length
    `focal` centred in the picture and no bend. The board's face then points along
    n = (nx, ny, nz), the camera looking along z, x to the picture's right and y
    down: the left side is turned away by atan(nx / nz), the top by atan(ny / nz).

    The method is SOLVEPNP_SQPNP, which needs 3 points or more. solvePnP's
    default needs 6 in OpenCV 5.0, and a frame with 4 corners found stopped the
    lens view (2026-11-02); SOLVEPNP_IPPE, the method for flat targets, missed by
    112 px on an exact projection of the board held upside down (OpenCV 5.0.0).
    None with fewer than 4 found, or when OpenCV finds no pose, so one poor frame
    never stops the view."""
    if ids is None or len(ids) < 4:
        return None
    layout = board.getChessboardCorners()[ids.ravel()].astype(np.float64)
    camera = np.array([[focal, 0, width / 2], [0, focal, height / 2], [0, 0, 1]])
    try:
        ok, rvec, _ = cv2.solvePnP(layout, corners.reshape(-1, 2).astype(np.float64), camera, None, flags=cv2.SOLVEPNP_SQPNP)
    except cv2.error:
        return None
    if not ok:
        return None
    nx, ny, nz = cv2.Rodrigues(rvec)[0][:, 2]
    # The board's z points into its face or out of it depending on how it is held;
    # taking nz away from the camera makes the answer the same either way.
    if nz < 0:
        nx, ny, nz = -nx, -ny, -nz
    across, down = math.degrees(math.atan2(nx, nz)), math.degrees(math.atan2(ny, nz))
    return {"left": across, "right": -across, "top": down, "bottom": -down}


def make_board(board: dict) -> cv2.aruco.CharucoBoard:
    """The printed board, at its measured size. The marker keeps its share of the
    square, since the print scales both alike."""
    square_m = board["printed_square_mm"] / 1_000
    marker_m = square_m * board["marker_mm"] / board["square_mm"]
    dictionary = cv2.aruco.getPredefinedDictionary(getattr(cv2.aruco, board["dictionary"]))
    return cv2.aruco.CharucoBoard((board["squares_x"], board["squares_y"]), square_m, marker_m, dictionary)


def is_monotonic(camera_matrix: np.ndarray, dist: np.ndarray, width: int, height: int) -> bool:
    """Whether the radial bend keeps growing out to the picture's corners. OpenCV's
    calib3d reference: radial distortion "is always monotonic for real lenses", and
    a result that is not "should be considered a calibration failure"."""
    fx, fy, cx, cy = camera_matrix[0, 0], camera_matrix[1, 1], camera_matrix[0, 2], camera_matrix[1, 2]
    corners = np.array([[0, 0], [width, 0], [0, height], [width, height]], dtype=np.float64)
    r_max = float(np.max(np.hypot((corners[:, 0] - cx) / fx, (corners[:, 1] - cy) / fy)))
    k1, k2, k3 = dist[0], dist[1], dist[4] if len(dist) > 4 else 0.0
    r = np.linspace(0.0, r_max, 1_000)
    bent = r * (1 + k1 * r**2 + k2 * r**4 + k3 * r**6)
    return bool((np.diff(bent) > 0).all())


def solve(board: cv2.aruco.CharucoBoard, views: list[dict], width: int, height: int) -> dict:
    """calibrateCamera on the kept stills. Each view holds the board's corners found
    in one still, `corners` as `(u, v)` and `ids` as the board's corner numbers."""
    object_points, image_points = [], []
    for view in views:
        corners = np.asarray(view["corners"], dtype=np.float32).reshape(-1, 1, 2)
        ids = np.asarray(view["ids"], dtype=np.int32).reshape(-1, 1)
        obj, img = board.matchImagePoints(corners, ids)
        object_points.append(obj)
        image_points.append(img)
    rms, camera_matrix, dist, rvecs, tvecs = cv2.calibrateCamera(object_points, image_points, (width, height), None, None)
    dist = dist.ravel()
    per_view = []
    for obj, img, rvec, tvec in zip(object_points, image_points, rvecs, tvecs):
        projected, _ = cv2.projectPoints(obj, rvec, tvec, camera_matrix, dist)
        per_view.append(float(np.sqrt(np.mean(np.sum((projected.reshape(-1, 2) - img.reshape(-1, 2)) ** 2, axis=1)))))
    return {
        "rms_px": float(rms),
        "camera_matrix": camera_matrix.tolist(),
        "dist": dist.tolist(),
        "monotonic": is_monotonic(camera_matrix, dist, width, height),
        "per_view_rms_px": per_view,
        "views": len(views),
        "width": width,
        "height": height,
    }
