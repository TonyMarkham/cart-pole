"""Makes the printable board that measures the camera's lens.

The lens bends the picture, most at the edges, and the matrix that turns pixels into
metres cannot follow a bend. Measuring the bend needs many points whose true layout
is known, seen all over the picture: a ChArUco board, a chessboard with an ArUco
marker in each white square, so OpenCV knows which corner is which even when only
part of the board is in view (OpenCV, "Calibration with ArUco and ChArUco").

Reads [lens_board] in settings.toml, and writes, into its out_dir:

- charuco-6x4-42mm.pdf (squares across, down, and their size): the page to print. The
  page is the board and a thin white border only, smaller than letter, so nothing
  reaches the printer's unprintable edge. Print it at 100%, "actual size", never
  "fit to page".
- the same name, .png: the same, as an image.

To check the print, measure a black square: it should be the size this prints.

Before writing, it finds the board in its own image, so a board OpenCV cannot read
is never printed.

Needs OpenCV and Pillow. On the PC, training/.venv has both.

usage (from __live__/orin/python): ../../../training/.venv/Scripts/python.exe src/charuco_board.py
"""

import sys
import tomllib
from pathlib import Path

import cv2
import numpy as np
from PIL import Image

SETTINGS = Path(__file__).resolve().parent.parent / "settings.toml"
MM_PER_INCH = 25.4


def dots(mm: float, dpi: int) -> int:
    return round(mm / MM_PER_INCH * dpi)


def main() -> int:
    board_settings = tomllib.loads(SETTINGS.read_text())["lens_board"]
    dpi = board_settings["dpi"]
    squares = (board_settings["squares_x"], board_settings["squares_y"])
    # Whole printer dots, so every square is the same size on paper.
    square_dots = dots(board_settings["square_mm"], dpi)
    marker_dots = dots(board_settings["marker_mm"], dpi)
    square_mm = square_dots / dpi * MM_PER_INCH
    marker_mm = marker_dots / dpi * MM_PER_INCH

    dictionary = cv2.aruco.getPredefinedDictionary(getattr(cv2.aruco, board_settings["dictionary"]))
    board = cv2.aruco.CharucoBoard(squares, square_mm / 1_000, marker_mm / 1_000, dictionary)
    board_image = board.generateImage((squares[0] * square_dots, squares[1] * square_dots), marginSize=0)
    border = dots(board_settings["border_mm"], dpi)
    page = cv2.copyMakeBorder(board_image, border, border, border, border, cv2.BORDER_CONSTANT, value=255)

    corners, ids, _, _ = cv2.aruco.CharucoDetector(board).detectBoard(page)
    inner = (squares[0] - 1) * (squares[1] - 1)
    found = 0 if ids is None else len(ids)
    if found != inner:
        print(f"the board's own image shows {found} of its {inner} corners: not written", file=sys.stderr)
        return 1

    out = SETTINGS.parent / board_settings["out_dir"]
    out.mkdir(exist_ok=True)
    # Pure black and white, so the PDF holds it losslessly (CCITT); a grey image is
    # stored as a JPEG, which blurs the very edges whose corners are measured.
    image = Image.fromarray(page).convert("1")
    name = f"charuco-{squares[0]}x{squares[1]}-{board_settings['square_mm']:g}mm"
    image.save(out / f"{name}.png", dpi=(dpi, dpi))
    image.save(out / f"{name}.pdf", resolution=dpi)
    width_mm, height_mm = (side / dpi * MM_PER_INCH for side in (page.shape[1], page.shape[0]))
    print(
        f"{out / name}.pdf: page {width_mm:.1f} x {height_mm:.1f} mm, {squares[0]} x {squares[1]} squares "
        f"of {square_mm:.3f} mm, markers {marker_mm:.3f} mm, all {inner} corners found in its own image"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
