"""The calibration: finds the matrix that turns pixels into metres, before training
starts (__documentation__/synthetic-loop-dia.md, Calibration, C5-C10).

This Orin never sees a capture. The orchestrator sends it the poses, each with Pivot's
and End's true (x, z) from MuJoCo. It renders each pose until enough images have both
markers, turns each marker's detections into one point, and at the end fits the matrix
to all the points and writes it where the training instances' Orins will read it.

Messages, control socket (orchestrator):
- in  text  {"type": "measure", "pose": name, "joints": [...], "pivot": [x, z],
             "end": [x, z]}                                     one pose (C6), with
             "cameraTransform" when the run has a camera pose, forwarded in
             each trigger (trigger.py)
- out text  {"type": "measured", "pose": name, "images": n, "renders": r}   (C7)
- in  text  {"type": "fit"}                                     (C8)
- out text  {"type": "fitted", "ok": bool, "worst_px": w, "lens_k": k}      (C9)

A pose whose markers are not seen in enough renders raises, which ends this process,
and the orchestrator fails the run: it cannot train without the matrix.
"""

import json
import tomllib
from pathlib import Path

from websockets.sync.client import ClientConnection

from detector import Detector, FrameError
from detector_config import DetectorConfig
from fit import Pair, fit, store
from lens import Optics
from measure import clipped_mean
from trigger import trigger


class CalibrationError(RuntimeError):
    """A pose could not be measured."""


def load_settings(path: Path) -> dict:
    """[calibration] of the Orin's settings file."""
    with open(path, "rb") as f:
        return tomllib.load(f)["calibration"]


def measure_pose(
    detector: Detector,
    camera: ClientConnection,
    message: dict,
    calibration: dict,
    slots: tuple[int, int],
    frame: tuple[int, int],
    first_id: int,
) -> tuple[list[Pair], int, int]:
    """Renders one pose until `images` frames have both Pivot and End. Returns the
    pose's two pairs, how many images counted, and how many renders that took."""
    pivot_slot, end_slot = slots
    width, height = frame
    pivots, ends = [], []
    renders = 0
    while len(pivots) < calibration["images"]:
        if renders == calibration["max_renders"]:
            raise CalibrationError(f"pose {message['pose']}: Pivot and End were seen together in only "
                                   f"{len(pivots)} of {renders} renders")
        camera.send(trigger(first_id + renders, message))
        jpeg = camera.recv()
        renders += 1
        if not isinstance(jpeg, bytes):
            raise CalibrationError(f"pose {message['pose']}: Unity sent text, expected the JPEG")
        try:
            found = detector.detect(jpeg).slots
        except FrameError as error:
            raise CalibrationError(f"pose {message['pose']}: {error}") from error
        pivot, end = found[pivot_slot], found[end_slot]
        # Only an image with both counts, so both points come from the same images.
        if pivot != (-1.0, -1.0) and end != (-1.0, -1.0):
            pivots.append((pivot[0] * width, pivot[1] * height))
            ends.append((end[0] * width, end[1] * height))
    pairs = []
    for marker, detections, metres in (("Pivot", pivots, message["pivot"]), ("End", ends, message["end"])):
        point = clipped_mean(detections, calibration["clip_k"])
        print(f"synthetic orin: {message['pose']}: {marker} at ({point.u:.3f}, {point.v:.3f}) px, "
              f"spread {point.spread:.3f} px, {point.dropped} of {point.count} dropped", flush=True)
        pairs.append(Pair(f"{marker} {message['pose']}", (point.u, point.v), (float(metres[0]), float(metres[1]))))
    return pairs, len(pivots), renders


def run(
    detector: Detector,
    config: DetectorConfig,
    ws: ClientConnection,
    camera: ClientConnection,
    settings_path: Path,
    frame: tuple[int, int],
    path: Path,
) -> None:
    """Answers the orchestrator's measures and its fit, until it closes the socket."""
    calibration = load_settings(settings_path)
    slots = (config.classes.index("Pivot"), config.classes.index("End"))
    pairs: list[Pair] = []
    next_id = 0
    for text in ws:
        if isinstance(text, bytes):
            raise CalibrationError("binary message on the control socket")
        message = json.loads(text)
        kind = message.get("type")
        if kind == "measure":
            found, images, renders = measure_pose(detector, camera, message, calibration, slots, frame, next_id)
            pairs += found
            next_id += renders
            ws.send(json.dumps({"type": "measured", "pose": message["pose"], "images": images, "renders": renders}))
        elif kind == "fit":
            report = fit(pairs, calibration, Optics.from_settings(calibration, frame))
            for residual in report["residuals"]:
                print(f"synthetic orin: residual {residual['point']}: {residual['px']:.3f} px", flush=True)
            print(f"synthetic orin: lens number {report['lens_k']:+.3f}, worst point {report['worst_px']:.3f} px "
                  f"({report['worst_uncorrected_px']:.3f} px with no lens number), limit {report['limit_px']} px",
                  flush=True)
            if report["ok"]:
                store(report, frame, path)
                print(f"synthetic orin: the matrix is in {path}", flush=True)
            ws.send(json.dumps({"type": "fitted", "ok": report["ok"], "worst_px": report["worst_px"],
                                "lens_k": report["lens_k"]}))
        else:
            raise CalibrationError(f"unexpected message on the control socket: {text!r}")
