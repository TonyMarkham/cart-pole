"""Measures the calibration's stops and fits the matrix, on request from the
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
from dataclasses import asdict
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

import cv2
import numpy as np

from camera import Camera, CameraError
from detector import NOT_DETECTED, Detector
from detector_config import DEFAULT_PATH, DetectorConfig
from fit import STOPS, fit, store
from lens import Optics
from measure import MarkerPoint, clipped_mean

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

IDLE, SETTLING, MEASURING, DONE, FAILED = "idle", "settling", "measuring", "done", "failed"


class Service:
    """One measurement at a time, on one thread that also owns the GPU. Everything
    the requests read is behind one lock, and the requests themselves never wait
    on the camera."""

    def __init__(self, settings: dict):
        self.settings = settings
        self.calibration = settings["calibration"]
        self.stream = settings["stream"]
        self._jobs: queue.Queue[tuple[int, str, int]] = queue.Queue()
        self._lock = threading.Lock()
        # Each start and cancel changes this, so a measurement that finds it changed
        # knows it has been cancelled, even if another has started since.
        self._generation = 0
        self._state = IDLE
        self._stop: str | None = None
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
                "target": self._target,
                "count": self._count,
                "message": self._message,
                "summary": self._summary,
                "frame": base64.b64encode(latest["jpeg"]).decode() if latest else None,
                "boxes": latest["boxes"] if latest else None,
                "width": self.stream["width"],
                "height": self.stream["height"],
                "measured": [stop for stop in STOPS if stop in self._accepted],
                "min_images": self.calibration["min_images"],
                "max_images": self.calibration["max_images"],
                "gaps_mm": dict(self._gaps_mm),
                "max_gap_mm": self.calibration["max_gap_mm"],
            }

    def start(self, stop: str, target: int, gap_mm: float | None) -> str | None:
        """Begins a measurement, or says why not. Each end needs its hand-measured
        gap, and the centre has none."""
        if stop not in STOPS:
            return f"no stop called {stop}"
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
            self._state, self._stop, self._target, self._count = SETTLING, stop, target, 0
            self._message, self._summary, self._points, self._latest = "", None, None, None
            self._gap_mm = None if stop == "centre" else gap_mm
            self._generation += 1
            self._jobs.put((self._generation, stop, target))
        return None

    def cancel(self) -> None:
        """Ends any measurement, and throws its result away."""
        with self._lock:
            self._generation += 1
            self._reset()

    def accept(self) -> str | None:
        """Keeps the finished measurement for its stop, replacing any earlier one."""
        with self._lock:
            if self._state != DONE or self._points is None or self._stop is None:
                return "there is no finished measurement to keep"
            self._accepted[self._stop] = self._points
            if self._gap_mm is not None:
                self._gaps_mm[self._stop] = self._gap_mm
            self._kept_at[self._stop] = time.time()
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
            "stops": {
                stop: {name: asdict(point) for name, point in points.items()}
                for stop, points in self._accepted.items()
            },
            "gaps_mm": self._gaps_mm,
            "kept_at": self._kept_at,
        }
        temporary = path.with_suffix(".tmp")
        temporary.write_text(json.dumps(record, indent=2))
        os.replace(temporary, path)

    def _load_kept(self) -> None:
        """Picks up what was kept before a restart. A file that cannot be read means
        the stops are measured again, so it must not stop the service starting."""
        path = self._kept_path()
        if not path.exists():
            return
        try:
            record = json.loads(path.read_text())
            self._accepted = {
                stop: {name: MarkerPoint(**point) for name, point in points.items()}
                for stop, points in record["stops"].items()
            }
            self._gaps_mm = record["gaps_mm"]
            self._kept_at = record["kept_at"]
        except (OSError, ValueError, KeyError, TypeError) as error:
            print(f"{path} could not be read, so the stops must be measured again: {error}", flush=True)

    def _reset(self) -> None:
        self._state, self._stop, self._target, self._count = IDLE, None, 0, 0
        self._message, self._summary, self._points, self._latest = "", None, None, None

    def fit(self, span_pulses: int) -> dict:
        # Read afresh, so a changed setting needs no restart.
        settings = tomllib.loads(DEFAULT_PATH.read_text())
        calibration = settings["calibration"]
        with self._lock:
            stops = dict(self._accepted)
            gaps_mm = dict(self._gaps_mm)
            kept_at = dict(self._kept_at)
        report = fit(stops, gaps_mm, calibration, span_pulses, Optics.from_settings(settings))
        now = time.time()
        report["kept_minutes_ago"] = {stop: round((now - kept_at[stop]) / 60) for stop in STOPS if stop in kept_at}
        report["saved_to"] = None
        if report["ok"]:
            path = Path(calibration["matrix_path"]).expanduser()
            store(report, stops, (self.stream["width"], self.stream["height"]), path)
            report["saved_to"] = str(path)
        return report

    def run(self) -> None:
        config = DetectorConfig.load()
        engine = Path(self.settings["detector"]["engine"]).expanduser()
        frame_size = (self.stream["width"], self.stream["height"])
        detector = Detector(config, engine, frame_size)
        print(f"detector loaded: {engine}", flush=True)
        while True:
            generation, stop, target = self._jobs.get()
            try:
                self._measure(detector, config, generation, stop, target)
            except (CameraError, OSError, RuntimeError, ValueError) as error:
                print(f"measure {stop}: {error}", flush=True)
                with self._lock:
                    if self._generation == generation:
                        self._state, self._message = FAILED, str(error)

    def _cancelled(self, generation: int) -> bool:
        with self._lock:
            return self._generation != generation

    def _measure(self, detector: Detector, config: DetectorConfig, generation: int, stop: str, target: int) -> None:
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
                # Only an image with both Pivot and End counts, so all three stops'
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
                "stop": stop,
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
                refused = service.start(query.get("stop", [""])[0], target, gap_mm)
                self._answer(409 if refused else 200, {"message": refused or "started"})
            elif url.path == "/cancel":
                service.cancel()
                self._answer(200, {"message": "cancelled"})
            elif url.path == "/accept":
                refused = service.accept()
                self._answer(409 if refused else 200, {"message": refused or "kept"})
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
