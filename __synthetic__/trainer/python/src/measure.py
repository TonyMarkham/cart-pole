"""Runs one measure (measures/) on a run folder and prints its table.

The measures' values (bands, thresholds, sample counts) are their tables in
the file settings.toml's [trainer] measures names.

usage (from the repo root):
  uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/measure.py NAME [RUN_FOLDER] [--settings FILE]
NAME is a file in __synthetic__/trainer/python/src/measures/ (episodes, entropy, actor_spread);
without RUN_FOLDER, the most recent run in [train] runs_dir that has a save.
"""

import argparse
import importlib
import pkgutil
import sys
import tomllib
from pathlib import Path

from run_folder import RunFolder
from settings import DEFAULT_PATH, Settings, read_train

SRC = Path(__file__).resolve().parent


def main() -> int:
    names = sorted(m.name for m in pkgutil.iter_modules([str(SRC / "measures")]) if m.name != "common")
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("name", choices=names)
    parser.add_argument("folder", type=Path, nargs="?")
    parser.add_argument("--settings", type=Path, default=DEFAULT_PATH)
    args = parser.parse_args()
    settings = Settings.load(args.settings)
    if settings.measures is None:
        raise SystemExit(f"{settings.path}: [trainer] measures names no file")
    with open(settings.measures, "rb") as f:
        tables = tomllib.load(f)
    if args.folder is not None:
        folder = RunFolder(args.folder.resolve())
    else:
        folder = RunFolder.latest(read_train(settings.gym).runs_dir)
    print(f"measure {args.name}: {folder.path}", flush=True)
    importlib.import_module(f"measures.{args.name}").run(folder, tables.get(args.name, {}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
