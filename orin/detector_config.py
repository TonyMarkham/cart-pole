"""The detector's settings, read from ObjectDetection.toml (synthetic-orin-plan.md R3).

One file configures detection for the Rust crate and for this project, so
both run the same model with the same letterbox and threshold. Only the keys
this project uses are read; the crate's own keys (edgetpu_model, threads)
are ignored here. Relative paths resolve against the file's folder, as the
crate resolves them.
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class DetectorConfig:
    model: Path  # the float ONNX the engine is built from
    input_size: int  # the model's square input, pixels
    pad_value: int  # letterbox fill, 0-255 grey
    confidence: float  # a class's best score counts as detected at >= this
    classes: list[str]  # class order = slot order = the model's class ids

    @staticmethod
    def load(path: Path) -> "DetectorConfig":
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        config = DetectorConfig(
            model=(path.parent / raw["model"]).resolve(),
            input_size=int(raw["input_size"]),
            pad_value=int(raw["pad_value"]),
            confidence=float(raw["confidence"]),
            classes=list(raw["classes"]),
        )
        # The marker frame has exactly four slots (synthetic-loop-dia.md, Contracts).
        if len(config.classes) != 4:
            raise ValueError(f"{path}: classes must name 4 slots, got {config.classes}")
        return config
