"""The detector's settings, read from [detector] in settings.toml.

One table holds the detector's parameters, so the engine build, the reference
check and whatever detects run the same model with the same letterbox and
threshold. A relative path resolves against the file's folder, and a path
starting with ~ against the home folder.
"""

from dataclasses import dataclass
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

# settings.toml, beside pyproject.toml.
DEFAULT_PATH = Path(__file__).resolve().parent.parent / "settings.toml"


@dataclass(frozen=True)
class DetectorConfig:
    model: Path  # the float ONNX the engine is built from
    input_size: int  # the model's square input, pixels
    pad_value: int  # letterbox fill, 0-255 grey
    confidence: float  # a class's best score counts as detected at >= this
    classes: list[str]  # class order = slot order = the model's class ids

    @staticmethod
    def load(path: Path = DEFAULT_PATH) -> "DetectorConfig":
        with open(path, "rb") as f:
            raw = tomllib.load(f)["detector"]
        config = DetectorConfig(
            model=(path.parent / Path(raw["model"]).expanduser()).resolve(),
            input_size=int(raw["input_size"]),
            pad_value=int(raw["pad_value"]),
            confidence=float(raw["confidence"]),
            classes=list(raw["classes"]),
        )
        # The marker frame has exactly four slots (__documentation__/synthetic-loop-dia.md, Contracts).
        if len(config.classes) != 4:
            raise ValueError(f"{path}: classes must name 4 slots, got {config.classes}")
        return config
