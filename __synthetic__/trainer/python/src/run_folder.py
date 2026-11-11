"""The run folder: the only module that knows a training run's layout
(rl-rewrite-plan.md, 3.2, and R7: a run is self-describing).

[train] runs_dir / <date>_<time>/ holds:

- latest.zip, latest_buffer.pkl and latest.json: the model, its replay
  buffer and its manifest at the last save, each written beside and then
  swapped in, so a crash mid-save keeps the previous one;
- model_<steps>.zip and model_<steps>.json: a snapshot and its manifest at
  every save;
- loop.log: the orchestrator's, the players' and the Orins' output;
- loop_from_<steps>/: the orchestrator's --out for the session that began
  at <steps> (0 for a new run), so a resume writes beside the sessions
  before it, never over them;
- captures_from_<steps>.csv: that session's capture record (record.py);
- episodes.csv: one row per finished episode.

The manifest beside every model holds what scored and shaped it: both toml
files as they were read, the four plugins' names and tables, the run's
values the orchestrator answered with (the cart top speed, the top pulse
rate and the timestep), the git commit (and whether the tree had
changes), and the MJCF's path and modification time. A resume reads it and refuses what cannot continue
(train.py).

A save is written on a second thread while training goes on (Saver), so
the loop does not stall while a gigabyte of buffer reaches the disk.
"""

import io
import json
import os
import shutil
import subprocess
import threading
import time
from datetime import datetime
from pathlib import Path

from stable_baselines3.common.save_util import save_to_pkl

from buffer import frozen
from contracts import RunValues
from settings import Gym, Settings


class RunFolder:
    def __init__(self, path: Path) -> None:
        self.path = path

    @staticmethod
    def create(runs_dir: Path) -> "RunFolder":
        path = runs_dir / datetime.now().strftime("%Y%m%d_%H%M%S")
        path.mkdir(parents=True)
        return RunFolder(path)

    @staticmethod
    def latest(runs_dir: Path) -> "RunFolder":
        """The most recent run with a saved manifest. Folders are named
        <date>_<time>, so the names sort in time order."""
        saved = sorted(f for f in runs_dir.iterdir() if (f / "latest.json").is_file()) if runs_dir.is_dir() else []
        if not saved:
            raise SystemExit(f"train: no run in {runs_dir} has a save to resume from")
        return RunFolder(saved[-1])

    @property
    def model(self) -> Path:
        return self.path / "latest.zip"

    @property
    def buffer(self) -> Path:
        return self.path / "latest_buffer.pkl"

    @property
    def manifest(self) -> Path:
        return self.path / "latest.json"

    @property
    def log(self) -> Path:
        return self.path / "loop.log"

    @property
    def episodes(self) -> Path:
        return self.path / "episodes.csv"

    def loop_out(self, steps: int) -> Path:
        return self.path / f"loop_from_{steps}"

    def captures(self, steps: int) -> Path:
        return self.path / f"captures_from_{steps}.csv"

    def sessions(self) -> list[Path]:
        """The capture records, oldest session first."""
        return sorted(self.path.glob("captures_from_*.csv"), key=lambda p: int(p.stem.rsplit("_", 1)[1]))

    def write(self, model_zip: bytes, buffer, manifest: dict) -> None:
        """The model (SB3's zip, already made), its buffer and its manifest,
        then a snapshot of each."""
        steps = manifest["steps"]
        (self.path / "latest.saving.zip").write_bytes(model_zip)
        save_to_pkl(self.path / "latest_buffer.saving.pkl", buffer)
        (self.path / "latest.saving.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
        os.replace(self.path / "latest.saving.zip", self.model)
        os.replace(self.path / "latest_buffer.saving.pkl", self.buffer)
        os.replace(self.path / "latest.saving.json", self.manifest)
        shutil.copyfile(self.model, self.path / f"model_{steps}.zip")
        shutil.copyfile(self.manifest, self.path / f"model_{steps}.json")

    def read_manifest(self) -> dict:
        if not self.manifest.is_file():
            raise SystemExit(f"train: {self.path} has no latest.json: not a run this trainer saved")
        return json.loads(self.manifest.read_text(encoding="utf-8"))


class Saver:
    """Saves the run while training goes on. The training thread keeps only
    what training would change under the save: the model's zip, made in
    memory (the weights move at every learning step, about 3.7 MB), and the
    buffer frozen (buffer.frozen). A second thread writes the files. One
    save is written at a time: a save that finds the last one still writing
    waits for it, and says how long, so `rows_ahead`, the rows training
    can write between two saves, bounds what the frozen buffer must copy.
    A save that failed raises its error on the training thread at the next
    save or at wait()."""

    def __init__(self, folder: RunFolder, manifest, rows_ahead: int) -> None:
        self.folder = folder
        self.manifest = manifest  # steps -> the manifest to save beside the model
        self.rows_ahead = rows_ahead
        self.thread = None
        self.error = None

    def start(self, model, steps: int) -> None:
        began = time.perf_counter()
        self.wait()
        waited = time.perf_counter() - began
        if waited > 0.01:
            print(f"train: training waited {waited:.1f} s for the last save to finish", flush=True)
        model_zip = io.BytesIO()
        model.save(model_zip)
        buffer = frozen(model.replay_buffer, self.rows_ahead)
        self.thread = threading.Thread(target=self.write, args=(model_zip.getvalue(), buffer, steps),
                                       name="save")
        self.thread.start()

    def write(self, model_zip: bytes, buffer, steps: int) -> None:
        began = time.perf_counter()
        try:
            self.folder.write(model_zip, buffer, self.manifest(steps))
        except BaseException as error:  # raised on the training thread by wait()
            self.error = error
            return
        print(f"train: saved at step {steps} -> {self.folder.path} "
              f"({time.perf_counter() - began:.1f} s on the save thread)", flush=True)

    def wait(self) -> None:
        if self.thread is not None:
            self.thread.join()
            self.thread = None
        if self.error is not None:
            error, self.error = self.error, None
            raise error


def git(repo: Path) -> dict:
    """The commit the code ran at, and whether the tree had changes."""
    def run(*args: str) -> str:
        return subprocess.run(["git", *args], cwd=repo, capture_output=True, text=True, check=True).stdout.strip()
    try:
        return {"commit": run("rev-parse", "HEAD"), "dirty": bool(run("status", "--porcelain"))}
    except (OSError, subprocess.CalledProcessError) as error:
        return {"commit": None, "error": str(error)}


def plugins_of(gym: Gym) -> dict:
    def entry(part):
        return {"name": part.name, "table": part.table}
    return {"observation": entry(gym.observation), "action": entry(gym.action), "reward": entry(gym.reward),
            "end": [entry(p) for p in gym.endings]}


def build_manifest(settings: Settings, gym: Gym, values: RunValues, steps: int, instances: int) -> dict:
    model = settings.mjcf.model
    return {
        "steps": steps,
        "instances": instances,
        "plugins": plugins_of(gym),
        "run_values": values.message(),
        "settings_toml": settings.path.read_text(encoding="utf-8"),
        "gym_toml": gym.text,
        "git": git(Path(__file__).resolve().parent),
        "mjcf": {"path": str(model),
                 "modified": datetime.fromtimestamp(model.stat().st_mtime).isoformat() if model.is_file() else None},
    }


def refusal(saved: dict, now: dict) -> str | None:
    """Why a run saved with manifest `saved` cannot continue under `now`,
    or None. The observation is the networks' input; the end conditions
    decide where stored episodes ended, which no rescore can move; the run's
    values set the observation's bounds (the timestep) and the action's
    scale (the top pulse rate), so other values would make the stored
    actions mean other pulse rates; a run saved with command rates, before
    the rig's timings (__documentation__/synthetic-loop-dia.md, Timing, 2026-11-08), held each
    stored action for a command rate's gap, which no longer exists.
    The reward may change: a resume rescores it (train.py)."""
    if "command_rates" in saved:
        return (f"the run was saved with the command rates {saved['command_rates']}, before the rig's timings "
                "replaced them (__documentation__/synthetic-loop-dia.md, Timing): each stored action was held for another time, "
                "so start a new run")
    if saved.get("run_values") != now["run_values"]:
        return (f"the run was saved with the run's values {saved.get('run_values')} and the orchestrator now "
                f"answers {now['run_values']}: the observation's bounds and the "
                f"action's scale follow them, so start a new run")
    was, will = saved["plugins"], now["plugins"]
    if was["observation"] != will["observation"]:
        return (f"the run was saved with observation {was['observation']} and the gym file now gives "
                f"{will['observation']}: the observation is the networks' input, so start a new run")
    if was["end"] != will["end"]:
        return (f"the run was saved with end conditions {was['end']} and the gym file now gives {will['end']}: "
                f"a rescore cannot move where stored episodes ended, so start a new run")
    return None
