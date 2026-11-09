"""Measures the calibration's poses and fits the matrix, on request from the
webserver. A browser cannot run the camera or the GPU, and detection on the robot
is Python, so this small service owns both, and the Rust webserver forwards the
calibration page's requests here. It only ever watches: nothing here moves the cart.

Runs on the Orin only, and listens only to the Orin itself. Keep it running beside
the webserver; the engine loads once when it starts. The camera is opened only for
the length of a measurement.

The settings are settings.toml, beside this folder.

usage (from __live__/orin/python): .venv/bin/python src/calibrate_service.py
"""

import base64
import json
import os
import queue
import sys
import threading
import time
from collections import deque
from dataclasses import asdict
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import cv2
import numpy as np

from camera import Camera, CameraError
from detector import NOT_DETECTED, Detector
from detector_config import DEFAULT_PATH, DetectorConfig
from fit import POSE_KEYS, POSES, fit, pose_key, store
from lens_calibration import Position, focal_px, make_board, outline, plan, solve, turn
from lens import Lens
from measure import MarkerPoint, clipped_mean

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

IDLE, SETTLING, MEASURING, DONE, FAILED = "idle", "settling", "measuring", "done", "failed"

# The kinds of job the camera thread runs, one at a time.
MEASURE, LIVE, LENS = "measure", "live", "lens"


class Service:
    """One measurement at a time, on one thread that also owns the GPU. Everything
    the requests read is behind one lock, and the requests themselves never wait
    on the camera."""

    def __init__(self, settings: dict):
        self.settings = settings
        self.calibration = settings["calibration"]
        self.stream = settings["stream"]
        self.live = settings["live"]
        # (kind, generation, pose, target); a live job has no pose or target.
        self._jobs: queue.Queue[tuple[str, int, str, int]] = queue.Queue()
        self._lock = threading.Lock()
        # Each start and cancel changes this, so a measurement that finds it changed
        # knows it has been cancelled, even if another has started since.
        self._generation = 0
        self._state = IDLE
        self._stop: str | None = None
        self._pose: str | None = None
        self._target = 0
        self._count = 0
        self._message = ""
        self._summary: dict | None = None
        self._points: dict[str, MarkerPoint] | None = None
        self._latest: dict | None = None
        self._gap_mm: float | None = None
        self._accepted: dict[str, dict[str, MarkerPoint]] = {}
        self._gaps_mm: dict[str, float] = {}
        self._kept_at: dict[str, float] = {}
        # The live view: whether it is wanted, when a page last looked, and what the
        # latest frames showed.
        self._live_on = False
        self._live_asked = 0.0
        self._live_latest: dict | None = None
        self._live_found: dict[str, deque] = {}
        self._live_frames = 0
        self._live_per_s = 0.0
        self._live_message = ""
        # The lens measurement: the full-sensor frame, the positions to hold the board
        # at, the camera's scale as a focal length for telling how far the board is
        # turned, the stills kept so far and which position is next. The stills are
        # saved as they are kept, so a restart loses none.
        self.lens = settings["lens"]
        self.lens_size = (settings["camera"]["width"], settings["camera"]["height"])
        self.lens_board = make_board(settings["lens_board"])
        self.lens_positions = plan(self.lens, settings["lens_board"], *self.lens_size)
        self.lens_focal = focal_px(self.lens, settings["lens_board"])
        self.lens_all_corners = len(self.lens_board.getChessboardCorners())
        self._lens_on = False
        self._lens_asked = 0.0
        self._lens_latest: dict | None = None
        self._lens_per_s = 0.0
        self._lens_held = 0
        self._lens_message = ""
        self._lens_views: list[dict] = []
        self._lens_next = 0
        self._lens_result: dict | None = None
        self._load_lens_views()
        self._load_kept()

    def is_busy(self) -> bool:
        with self._lock:
            return self._state in (SETTLING, MEASURING)

    def status(self) -> dict:
        with self._lock:
            latest = self._latest
            return {
                "state": self._state,
                "stop": self._stop,
                "pose": self._pose,
                "target": self._target,
                "count": self._count,
                "message": self._message,
                "summary": self._summary,
                "frame": base64.b64encode(latest["jpeg"]).decode() if latest else None,
                "boxes": latest["boxes"] if latest else None,
                "width": self.stream["width"],
                "height": self.stream["height"],
                "poses": list(POSE_KEYS),
                "measured": [key for key in POSE_KEYS if key in self._accepted],
                "min_images": self.calibration["min_images"],
                "max_images": self.calibration["max_images"],
                "gaps_mm": dict(self._gaps_mm),
                "max_gap_mm": self.calibration["max_gap_mm"],
            }

    def start(self, stop: str, pole: str, target: int, gap_mm: float | None) -> str | None:
        """Begins a pose's measurement, or says why not. Each end needs its
        hand-measured gap, whatever the pole does, and the centre has none."""
        if (stop, pole) not in POSES:
            return f"no pose with the cart at {stop!r} and the pole {pole!r}"
        if stop != "centre":
            most = self.calibration["max_gap_mm"]
            if gap_mm is None or not 0 <= gap_mm <= most:
                return f"the gap must be a number of millimetres from 0 to {most}"
        low, high = self.calibration["min_images"], self.calibration["max_images"]
        if not low <= target <= high:
            return f"the number of images must be from {low} to {high}"
        with self._lock:
            if self._state in (SETTLING, MEASURING):
                return "a measurement is already under way"
            # The calibration page watches the live view until its Calibrate now, so a
            # measurement takes the camera from it. The camera thread runs one job at a
            # time, so the measurement opens the camera once the live view has let go.
            if self._live_on:
                self._live_on, self._live_message = False, "a measurement started"
            if self._lens_on:
                self._lens_on, self._lens_message = False, "a measurement started"
            self._state, self._stop, self._target, self._count = SETTLING, stop, target, 0
            self._pose = pose_key(stop, pole)
            self._message, self._summary, self._points, self._latest = "", None, None, None
            self._gap_mm = None if stop == "centre" else gap_mm
            self._generation += 1
            self._jobs.put((MEASURE, self._generation, self._pose, target))
        return None

    def start_live(self) -> str | None:
        """Starts the live view, or says why not. It runs until stopped, or until no
        page has looked for `idle_stop_s`."""
        with self._lock:
            if self._state in (SETTLING, MEASURING):
                return "a measurement is using the camera: wait for it to end"
            if self._live_on:
                return None
            if self._lens_on:
                self._lens_on, self._lens_message = False, "the live view started"
            self._live_on, self._live_asked = True, time.monotonic()
            self._live_latest, self._live_frames, self._live_per_s, self._live_message = None, 0, 0.0, ""
            self._live_found = {}
            self._generation += 1
            self._jobs.put((LIVE, self._generation, "", 0))
        return None

    def stop_live(self) -> None:
        with self._lock:
            self._live_on = False

    def live_status(self) -> dict:
        """The latest frame and its boxes, and how often each marker was found in the
        last `window_frames` frames. Asking counts as watching."""
        with self._lock:
            self._live_asked = time.monotonic()
            latest = self._live_latest
            return {
                "on": self._live_on,
                "message": self._live_message,
                "frames": self._live_frames,
                "per_s": self._live_per_s,
                "window": self.live["window_frames"],
                "found": {name: sum(flags) for name, flags in self._live_found.items()},
                "frame": base64.b64encode(latest["jpeg"]).decode() if latest else None,
                "boxes": latest["boxes"] if latest else None,
                "width": self.stream["width"],
                "height": self.stream["height"],
            }

    def start_lens(self) -> str | None:
        """Starts the lens measurement's view, or says why not. Kept stills and the
        next position carry on from where they were."""
        with self._lock:
            if self._state in (SETTLING, MEASURING):
                return "a measurement is using the camera: wait for it to end"
            if self._lens_on:
                return None
            if self._live_on:
                self._live_on, self._live_message = False, "the lens measurement started"
            self._lens_on, self._lens_asked = True, time.monotonic()
            self._lens_latest, self._lens_per_s, self._lens_held, self._lens_message = None, 0.0, 0, ""
            self._generation += 1
            self._jobs.put((LENS, self._generation, "", 0))
        return None

    def stop_lens(self) -> None:
        with self._lock:
            self._lens_on = False

    def lens_skip(self) -> None:
        """Moves the rectangle on without keeping a still, for a position the board
        cannot reach."""
        with self._lock:
            self._lens_next = self._next_position(self._lens_next + 1)
            self._lens_held = 0
            self._save_lens_views()

    def lens_restart(self) -> None:
        """Forgets every kept still, and starts again from the first position."""
        with self._lock:
            self._lens_views, self._lens_next, self._lens_held = [], 0, 0
            self._save_lens_views()

    def lens_still(self) -> str:
        """Saves the current frame at full size twice, for looking at, not for the
        lens: it keeps no still. Once with the box, the board's shape and the markers
        drawn in as the page shows them, so someone not at the tablet can see it, and
        once plain, with nothing drawn, so the detector can be tried on it."""
        with self._lock:
            latest = self._lens_latest
            position = self._lens_position()
        if latest is None:
            raise ValueError("there is no frame yet: press Start first")
        picture = cv2.cvtColor(latest["grey"], cv2.COLOR_GRAY2BGR)
        if position is not None:
            rect = position.rect
            colour = (64, 204, 46) if latest["ready"] else (45, 45, 255)
            cv2.rectangle(picture, (rect.x, rect.y), (rect.x + rect.w, rect.y + rect.h), colour, 6)
            cv2.polylines(picture, [np.asarray(position.shape, dtype=np.int32)], True, (255, 255, 255), 2)
        for quad in latest["markers"]:
            cv2.polylines(picture, [np.asarray(quad, dtype=np.int32)], True, (0, 212, 255), 3)
        if latest["outline"] is not None:
            cv2.polylines(picture, [np.asarray(latest["outline"], dtype=np.int32)], True, (255, 160, 0), 3)
        found = (
            f"{len(latest['markers'])} of {len(self.lens_board.getIds())} markers, "
            f"{latest['corners']} of {self.lens_all_corners} corners"
        )
        cv2.putText(picture, found, (20, self.lens_size[1] - 30), cv2.FONT_HERSHEY_SIMPLEX, 1.5, (0, 212, 255), 3)
        folder = Path(self.lens["stills_dir"]).expanduser()
        folder.mkdir(parents=True, exist_ok=True)
        # To the millisecond, so presses in quick succession each keep their own file.
        now = time.time()
        stamp = time.strftime("%Y%m%d-%H%M%S", time.gmtime(now))
        name = f"still-{stamp}-{int(now * 1000) % 1000:03d}"
        for suffix, image, quality in (("", picture, 90), ("-plain", latest["grey"], 100)):
            ok, encoded = cv2.imencode(".jpg", image, [cv2.IMWRITE_JPEG_QUALITY, quality])
            if not ok:
                raise RuntimeError("the still did not encode as a JPEG")
            (folder / f"{name}{suffix}.jpg").write_bytes(encoded.tobytes())
        return f"{name}.jpg and {name}-plain.jpg"

    def lens_solve(self) -> dict:
        """calibrateCamera on the kept stills; stores the result and a dated copy."""
        with self._lock:
            views = list(self._lens_views)
        least = self.lens["min_views"]
        if len(views) < least:
            raise ValueError(f"keep at least {least} stills first: {len(views)} so far")
        result = solve(self.lens_board, views, *self.lens_size)
        result["date"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        result["board"] = {"printed_square_mm": self.settings["lens_board"]["printed_square_mm"]}
        path = Path(self.lens["lens_path"]).expanduser()
        dated = path.parent / "saved" / f"lens-{time.strftime('%Y%m%d-%H%M%S', time.gmtime())}.json"
        for target in (path, dated):
            target.parent.mkdir(parents=True, exist_ok=True)
            temporary = target.with_suffix(".tmp")
            temporary.write_text(json.dumps(result, indent=2))
            os.replace(temporary, target)
        result["saved_to"] = str(path)
        with self._lock:
            self._lens_result = result
        return result

    def lens_status(self) -> dict:
        """The latest frame, the rectangle to fill and how long the board has been
        inside it. Asking counts as watching."""
        with self._lock:
            self._lens_asked = time.monotonic()
            latest = self._lens_latest
            position = self._lens_position()
            result = self._lens_result
            return {
                "on": self._lens_on,
                "message": self._lens_message,
                "per_s": self._lens_per_s,
                "frame": base64.b64encode(latest["jpeg"]).decode() if latest else None,
                "markers": latest["markers"] if latest else [],
                "outline": latest["outline"] if latest else None,
                "corners": latest["corners"] if latest else 0,
                "all_corners": self.lens_all_corners,
                "min_corners": self.lens["min_corners"],
                "in_box": latest["in_box"] if latest else False,
                "turn_deg": latest["turn_deg"] if latest else None,
                "ready": latest["ready"] if latest else False,
                "width": self.lens_size[0],
                "height": self.lens_size[1],
                "distance_mm": self.lens["distance_mm"],
                "tilt_deg": self.lens["tilt_deg"],
                "min_tilt_deg": self.lens["min_tilt_deg"],
                "rect": position.to_json() if position else None,
                "position": self._lens_next,
                "positions": len(self.lens_positions),
                "flat_positions": sum(1 for each in self.lens_positions if not each.tilt),
                "held": self._lens_held,
                "hold_frames": self.lens["hold_frames"],
                "kept": len(self._lens_views),
                "min_views": self.lens["min_views"],
                "result": None if result is None else {
                    key: result[key] for key in ("rms_px", "camera_matrix", "dist", "monotonic", "views", "saved_to")
                },
            }

    def _lens_position(self) -> Position | None:
        """Where to hold the board now, or None once every position is done. Called
        with the lock held."""
        return self.lens_positions[self._lens_next] if self._lens_next < len(self.lens_positions) else None

    def _next_position(self, start: int) -> int:
        """The first position from `start` with no kept still, or the count when none
        is left."""
        kept = {view["position"] for view in self._lens_views}
        index = start
        while index < len(self.lens_positions) and index in kept:
            index += 1
        return index

    def _lens_views_path(self) -> Path:
        return Path(self.lens["views_path"]).expanduser()

    def _save_lens_views(self) -> None:
        path = self._lens_views_path()
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_suffix(".tmp")
        temporary.write_text(json.dumps({"views": self._lens_views, "next": self._lens_next}))
        os.replace(temporary, path)

    def _load_lens_views(self) -> None:
        """Picks up the stills kept before a restart. One that cannot be read starts
        the measurement again, so it must not stop the service starting."""
        path = self._lens_views_path()
        if not path.exists():
            return
        try:
            record = json.loads(path.read_text())
            self._lens_views, self._lens_next = record["views"], record["next"]
        except (OSError, ValueError, KeyError, TypeError) as error:
            self._lens_views, self._lens_next = [], 0
            print(f"{path} could not be read, so the lens stills must be taken again: {error}", flush=True)

    def cancel(self) -> None:
        """Ends any measurement, and throws its result away."""
        with self._lock:
            self._generation += 1
            self._reset()

    def accept(self) -> str | None:
        """Keeps the finished measurement for its pose, replacing any earlier one. The
        gap belongs to the stop, so the latest typed at an end is the one kept."""
        with self._lock:
            if self._state != DONE or self._points is None or self._stop is None or self._pose is None:
                return "there is no finished measurement to keep"
            self._accepted[self._pose] = self._points
            if self._gap_mm is not None:
                self._gaps_mm[self._stop] = self._gap_mm
            self._kept_at[self._pose] = time.time()
            self._save_kept()
            self._reset()
        return None

    def _kept_path(self) -> Path:
        return Path(self.calibration["kept_path"]).expanduser()

    def _save_kept(self) -> None:
        """Written beside the target first and then moved over it, so a failure
        midway never loses what was kept before."""
        path = self._kept_path()
        path.parent.mkdir(parents=True, exist_ok=True)
        record = {
            "poses": {
                key: {name: asdict(point) for name, point in points.items()}
                for key, points in self._accepted.items()
            },
            "gaps_mm": self._gaps_mm,
            "kept_at": self._kept_at,
        }
        temporary = path.with_suffix(".tmp")
        temporary.write_text(json.dumps(record, indent=2))
        os.replace(temporary, path)

    def _load_kept(self) -> None:
        """Picks up what was kept before a restart. A file that cannot be read means
        the poses are measured again, so it must not stop the service starting. A file
        from before the seven poses has no "poses", and is not read."""
        path = self._kept_path()
        if not path.exists():
            return
        try:
            record = json.loads(path.read_text())
            self._accepted = {
                key: {name: MarkerPoint(**point) for name, point in points.items()}
                for key, points in record["poses"].items()
            }
            self._gaps_mm = record["gaps_mm"]
            self._kept_at = record["kept_at"]
        except (OSError, ValueError, KeyError, TypeError) as error:
            self._accepted, self._gaps_mm, self._kept_at = {}, {}, {}
            print(f"{path} could not be read, so the poses must be measured again: {error}", flush=True)

    def _reset(self) -> None:
        self._state, self._stop, self._pose, self._target, self._count = IDLE, None, None, 0, 0
        self._message, self._summary, self._points, self._latest = "", None, None, None

    def fit(self, span_pulses: int) -> dict:
        # Read afresh, so a changed setting needs no restart.
        settings = tomllib.loads(DEFAULT_PATH.read_text())
        calibration = settings["calibration"]
        with self._lock:
            poses = dict(self._accepted)
            gaps_mm = dict(self._gaps_mm)
            kept_at = dict(self._kept_at)
        # The lens calibration is read afresh too, so a new Solve on the /lens page is used
        # at the next fit. It is scaled to the detector's frames, which the points are in.
        frame = (settings["stream"]["width"], settings["stream"]["height"])
        lens = Lens.load(Path(settings["lens"]["lens_path"]).expanduser(), frame)
        report = fit(poses, gaps_mm, calibration, span_pulses, lens)
        now = time.time()
        report["kept_minutes_ago"] = {key: round((now - kept_at[key]) / 60) for key in POSE_KEYS if key in kept_at}
        # Every fit is kept, within the limit or not (user, 2026-11-02): "within_limit"
        # and "accepted" in the file say which, so whatever uses the matrix checks
        # them. The latest is matrix_path; a dated copy of each goes to saved/ beside
        # it and is never overwritten.
        path = Path(calibration["matrix_path"]).expanduser()
        dated = path.parent / "saved" / f"homography-{time.strftime('%Y%m%d-%H%M%S', time.gmtime())}.json"
        report["saved_to"] = str(path)
        report["dated_copy"] = str(dated)
        for target in (path, dated):
            store(report, poses, (self.stream["width"], self.stream["height"]), target)
        return report

    def accept_fit(self) -> str:
        """Marks the latest fit as the one to use although a point is over
        residual_px: whether the matrix is good enough is settled by the robot
        balancing with it, not by the limit (user, 2026-11-02). Marks its dated copy
        too, so the record of the decision is never lost."""
        settings = tomllib.loads(DEFAULT_PATH.read_text())
        path = Path(settings["calibration"]["matrix_path"]).expanduser()
        try:
            record = json.loads(path.read_text())
        except FileNotFoundError:
            raise ValueError("there is no fit to accept yet: measure the seven poses first") from None
        record["accepted"] = True
        record["accepted_at"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        targets = [path] + ([Path(record["dated_copy"])] if record.get("dated_copy") else [])
        for target in targets:
            temporary = target.with_suffix(".tmp")
            temporary.write_text(json.dumps(record, indent=2))
            os.replace(temporary, target)
        return f"Accepted: {path} is marked as the one to use, worst point {record['worst_px']:.3f} px."

    def run(self) -> None:
        config = DetectorConfig.load()
        engine = Path(self.settings["detector"]["engine"]).expanduser()
        frame_size = (self.stream["width"], self.stream["height"])
        detector = Detector(config, engine, frame_size)
        print(f"detector loaded: {engine}", flush=True)
        while True:
            kind, generation, pose, target = self._jobs.get()
            if kind == LENS:
                try:
                    self._lens(generation)
                except (CameraError, OSError, RuntimeError, ValueError, cv2.error) as error:
                    print(f"lens view: {error}", flush=True)
                    with self._lock:
                        self._lens_on, self._lens_message = False, str(error)
                continue
            if kind == LIVE:
                try:
                    self._live(detector, config, generation)
                except (CameraError, OSError, RuntimeError, ValueError) as error:
                    print(f"live view: {error}", flush=True)
                    with self._lock:
                        self._live_on, self._live_message = False, str(error)
                continue
            try:
                self._measure(detector, config, generation, pose, target)
            except (CameraError, OSError, RuntimeError, ValueError) as error:
                print(f"measure {pose}: {error}", flush=True)
                with self._lock:
                    if self._generation == generation:
                        self._state, self._message = FAILED, str(error)

    def _live(self, detector: Detector, config: DetectorConfig, generation: int) -> None:
        """Detects on every frame of the stream until the view is stopped or no page
        has looked for `idle_stop_s`. Every marker is drawn, not only Pivot and End,
        because the point is to see that the whole rig is in the picture. A view
        stopped and started again quickly is a new job: this one ends."""
        stream = self.stream
        width, height = stream["width"], stream["height"]
        window, idle_s = self.live["window_frames"], self.live["idle_stop_s"]
        found = {name: deque(maxlen=window) for name in config.classes}
        times: deque[float] = deque(maxlen=window)
        with Camera(self.settings["camera"], stream) as camera:
            for index, pixels in enumerate(camera.frames()):
                with self._lock:
                    if self._live_on and time.monotonic() - self._live_asked > idle_s:
                        self._live_on = False
                        self._live_message = f"stopped: no page looked for {idle_s} s"
                    if not self._live_on or self._generation != generation:
                        return
                if index < stream["settle_frames"]:
                    continue
                grey = np.frombuffer(pixels, np.uint8).reshape(height, width)
                ok, encoded = cv2.imencode(".jpg", grey, [cv2.IMWRITE_JPEG_QUALITY, stream["jpeg_quality"]])
                if not ok:
                    raise RuntimeError("a frame did not encode as a JPEG")
                jpeg = encoded.tobytes()
                frame = detector.detect(jpeg)
                times.append(time.perf_counter())
                boxes = {}
                for slot, name in enumerate(config.classes):
                    hit = frame.slots[slot] != NOT_DETECTED
                    found[name].append(hit)
                    boxes[name] = None if not hit else {
                        "cx": frame.slots[slot][0] * width,
                        "cy": frame.slots[slot][1] * height,
                        "w": frame.sizes[slot][0] * width,
                        "h": frame.sizes[slot][1] * height,
                    }
                per_s = (len(times) - 1) / (times[-1] - times[0]) if len(times) > 1 else 0.0
                with self._lock:
                    self._live_latest = {"jpeg": jpeg, "boxes": boxes}
                    self._live_found = {name: list(flags) for name, flags in found.items()}
                    self._live_frames += 1
                    self._live_per_s = per_s

    def _lens(self, generation: int) -> None:
        """Finds the board in every full-sensor frame. A frame counts when at least
        `min_corners` of the board's corners are found, everything found is inside
        the box, and, at a tilted position, the named side is turned at least
        `min_tilt_deg` away from the camera. Once `hold_frames` frames in a row count, that
        frame's corners are kept as a still and the box moves to the next position."""
        stream = dict(self.stream, width=self.lens_size[0], height=self.lens_size[1])
        width, height = self.lens_size
        detector = cv2.aruco.CharucoDetector(self.lens_board)
        tiles = self.lens["clahe_tiles"]
        clahe = cv2.createCLAHE(clipLimit=self.lens["clahe_clip_limit"], tileGridSize=(tiles, tiles))
        least = self.lens["min_corners"]
        idle_s = self.lens["idle_stop_s"]
        times: deque[float] = deque(maxlen=60)
        with Camera(self.settings["camera"], stream) as camera:
            for index, pixels in enumerate(camera.frames()):
                with self._lock:
                    if self._lens_on and time.monotonic() - self._lens_asked > idle_s:
                        self._lens_on = False
                        self._lens_message = f"stopped: no page looked for {idle_s} s"
                    if not self._lens_on or self._generation != generation:
                        return
                    position = self._lens_position()
                if index < stream["settle_frames"]:
                    continue
                # Contrast stretched before the board is looked for; the page and the
                # stills show this same frame, so they show what the detector saw.
                grey = clahe.apply(np.frombuffer(pixels, np.uint8).reshape(height, width))
                corners, ids, marker_corners, marker_ids = detector.detectBoard(grey)
                times.append(time.perf_counter())
                markers = [quad.reshape(-1, 2).tolist() for quad in marker_corners] if marker_ids is not None else []
                found = 0 if ids is None else len(ids)
                # One missed marker costs up to 4 corners, not the whole frame: a corner
                # is found only with both markers beside it (CharucoParameters.minMarkers,
                # 2 by default), and a marker in the board's middle touches 4 corners.
                in_box = bool(
                    position is not None and found >= least
                    and position.rect.holds(np.concatenate([corners.reshape(-1, 2), *(quad.reshape(-1, 2) for quad in marker_corners)]))
                )
                quad = outline(self.lens_board, corners, ids)
                # The named side's turn, for the page to show while the person turns
                # the board; None when flat or too few corners are found.
                sides = turn(self.lens_board, corners, ids, self.lens_focal, width, height) if position and position.tilt else None
                turn_deg = None if sides is None else round(sides[position.tilt], 1)
                turned = position is not None and (not position.tilt or (turn_deg is not None and turn_deg >= self.lens["min_tilt_deg"]))
                ready = in_box and turned
                # Shown at the detector's frame size, to keep the page's requests small;
                # every coordinate stays in full-sensor pixels.
                small = cv2.resize(grey, (stream["width"] // 2, stream["height"] // 2), interpolation=cv2.INTER_AREA)
                ok, encoded = cv2.imencode(".jpg", small, [cv2.IMWRITE_JPEG_QUALITY, 80])
                if not ok:
                    raise RuntimeError("a frame did not encode as a JPEG")
                with self._lock:
                    if self._generation != generation:
                        return
                    self._lens_held = self._lens_held + 1 if ready else 0
                    if self._lens_held >= self.lens["hold_frames"] and position is not None:
                        self._lens_views.append({
                            "position": self._lens_next,
                            "tilt": position.tilt,
                            "corners": corners.reshape(-1, 2).tolist(),
                            "ids": ids.ravel().tolist(),
                            "at": time.time(),
                        })
                        self._lens_next = self._next_position(self._lens_next + 1)
                        self._lens_held = 0
                        self._save_lens_views()
                    self._lens_latest = {
                        "jpeg": encoded.tobytes(),
                        "markers": markers,
                        "outline": None if quad is None else quad.round(1).tolist(),
                        "corners": found,
                        "in_box": in_box,
                        "turn_deg": turn_deg,
                        "ready": ready,
                        "grey": grey.copy(),
                    }
                    self._lens_per_s = (len(times) - 1) / (times[-1] - times[0]) if len(times) > 1 else 0.0

    def _cancelled(self, generation: int) -> bool:
        with self._lock:
            return self._generation != generation

    def _measure(self, detector: Detector, config: DetectorConfig, generation: int, pose: str, target: int) -> None:
        stream = self.stream
        width, height = stream["width"], stream["height"]
        detections: dict[str, list[tuple[float, float]]] = {name: [] for name in config.classes}
        slot_of = {name: config.classes.index(name) for name in config.classes}
        seen = 0
        count = 0
        started = time.perf_counter()
        with Camera(self.settings["camera"], stream) as camera:
            for index, pixels in enumerate(camera.frames()):
                if self._cancelled(generation):
                    return
                if index < stream["settle_frames"]:
                    started = time.perf_counter()
                    continue
                grey = np.frombuffer(pixels, np.uint8).reshape(height, width)
                ok, encoded = cv2.imencode(".jpg", grey, [cv2.IMWRITE_JPEG_QUALITY, stream["jpeg_quality"]])
                if not ok:
                    raise RuntimeError("a frame did not encode as a JPEG")
                jpeg = encoded.tobytes()
                frame = detector.detect(jpeg)
                seen += 1
                centres = {
                    name: None if frame.slots[slot] == NOT_DETECTED
                    else (frame.slots[slot][0] * width, frame.slots[slot][1] * height)
                    for name, slot in slot_of.items()
                }
                # Only an image with both Pivot and End counts, so all seven poses'
                # points come from images of the same quality.
                if centres["Pivot"] is not None and centres["End"] is not None:
                    count += 1
                    for name, centre in centres.items():
                        if centre is not None:
                            detections[name].append(centre)
                boxes = {
                    name: None if centres[name] is None else {
                        "cx": centres[name][0],
                        "cy": centres[name][1],
                        "w": frame.sizes[slot_of[name]][0] * width,
                        "h": frame.sizes[slot_of[name]][1] * height,
                    }
                    for name in ("Pivot", "End")
                }
                with self._lock:
                    if self._generation != generation:
                        return
                    self._state, self._count = MEASURING, count
                    self._latest = {"jpeg": jpeg, "boxes": boxes}
                if count >= target:
                    break
            # Before the camera shuts down, which takes a few seconds of its own.
            took_s = time.perf_counter() - started
        clip_k = self.calibration["clip_k"]
        points = {name: clipped_mean(found, clip_k) for name, found in detections.items() if found}
        with self._lock:
            if self._generation != generation:
                return
            self._state, self._points = DONE, points
            self._summary = {
                "pose": pose,
                "gap_mm": self._gap_mm,
                "images": count,
                "seen": seen,
                "took_s": took_s,
                "pivot": points["Pivot"].to_json(),
                "end": points["End"].to_json(),
            }


def make_handler(service: Service) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        def _answer(self, status: int, body: dict) -> None:
            data = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

        def do_GET(self) -> None:
            path = urlparse(self.path).path
            if path == "/status":
                self._answer(200, service.status())
            elif path == "/busy":
                self._answer(200, {"busy": service.is_busy()})
            elif path == "/live/status":
                self._answer(200, service.live_status())
            elif path == "/lens/status":
                self._answer(200, service.lens_status())
            else:
                self._answer(404, {"message": f"no route {path}"})

        def do_POST(self) -> None:
            url = urlparse(self.path)
            query = parse_qs(url.query)
            if url.path == "/measure":
                try:
                    target = int(query.get("count", [""])[0])
                except ValueError:
                    self._answer(400, {"message": "the number of images must be a whole number"})
                    return
                try:
                    gap_mm = float(query["gap_mm"][0]) if "gap_mm" in query else None
                except ValueError:
                    gap_mm = None
                refused = service.start(query.get("stop", [""])[0], query.get("pole", [""])[0], target, gap_mm)
                self._answer(409 if refused else 200, {"message": refused or "started"})
            elif url.path == "/live/start":
                refused = service.start_live()
                self._answer(409 if refused else 200, {"message": refused or "started"})
            elif url.path == "/live/stop":
                service.stop_live()
                self._answer(200, {"message": "stopped"})
            elif url.path == "/lens/start":
                refused = service.start_lens()
                self._answer(409 if refused else 200, {"message": refused or "started"})
            elif url.path == "/lens/stop":
                service.stop_lens()
                self._answer(200, {"message": "stopped"})
            elif url.path == "/lens/skip":
                service.lens_skip()
                self._answer(200, {"message": "skipped"})
            elif url.path == "/lens/still":
                try:
                    self._answer(200, {"message": f"saved as {service.lens_still()}"})
                except (ValueError, RuntimeError, OSError) as error:
                    self._answer(409, {"message": str(error)})
            elif url.path == "/lens/restart":
                service.lens_restart()
                self._answer(200, {"message": "every kept still forgotten"})
            elif url.path == "/lens/solve":
                try:
                    self._answer(200, service.lens_solve())
                except (ValueError, cv2.error) as error:
                    self._answer(409, {"message": str(error)})
            elif url.path == "/cancel":
                service.cancel()
                self._answer(200, {"message": "cancelled"})
            elif url.path == "/accept":
                refused = service.accept()
                self._answer(409 if refused else 200, {"message": refused or "kept"})
            elif url.path == "/fit/accept":
                try:
                    said = service.accept_fit()
                except (ValueError, OSError) as error:
                    self._answer(409, {"message": str(error)})
                    return
                self._answer(200, {"message": said})
            elif url.path == "/fit":
                try:
                    report = service.fit(int(query.get("span", [""])[0]))
                except ValueError as error:  # FitError is one, and so is a bad span
                    self._answer(409, {"message": str(error)})
                    return
                self._answer(200, report)
            else:
                self._answer(404, {"message": f"no route {url.path}"})

        def log_message(self, *_) -> None:
            """Status is asked for several times a second: not worth a line each."""

    return Handler


def main() -> int:
    settings = tomllib.loads(DEFAULT_PATH.read_text())
    service = Service(settings)
    threading.Thread(target=service.run, daemon=True).start()
    address = (settings["service"]["host"], settings["service"]["port"])
    server = ThreadingHTTPServer(address, make_handler(service))
    print(f"calibrate service at http://{address[0]}:{address[1]}", flush=True)
    server.serve_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
