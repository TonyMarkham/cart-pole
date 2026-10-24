"""The trainer's settings, read from trainer/settings.toml.

The trainer starts each run by launching the orchestrator
(synthetic-loop-dia.md, 1), so the command that launches it is the setting
it cannot run without; [mjcf] names the model that run loads (2). Relative
paths resolve against the file's folder, so the trainer runs the same from
any working directory.
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path

# trainer/settings.toml, beside pyproject.toml.
DEFAULT_PATH = Path(__file__).resolve().parent.parent / "settings.toml"


@dataclass(frozen=True)
class Mjcf:
    model: Path  # the MJCF the run loads (absolute)
    cart_joint: str  # as in the model's <joint name>; also a wire key
    pole_joint: str
    actuator: str  # as in the model's actuator name

    def arguments(self) -> list[str]:
        return [
            "--model", str(self.model),
            "--cart-joint", self.cart_joint,
            "--pole-joint", self.pole_joint,
            "--actuator", self.actuator,
        ]


@dataclass(frozen=True)
class Settings:
    orchestrator_command: list[str]  # the program (absolute), the ports and seed, the [mjcf] arguments, then the rest
    orchestrator_cwd: Path  # the folder the command runs in
    orin_port: int  # the control socket, 127.0.0.1
    unity_port: int  # the frame socket, 127.0.0.1
    seed: int  # the run's seed, 0 to 2**64 - 1
    rnd_delay_min: float  # the Orin latency's band, ms: RND(min, max)
    rnd_delay_max: float
    mjcf: Mjcf

    @staticmethod
    def load(path: Path = DEFAULT_PATH) -> "Settings":
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        orchestrator = raw["orchestrator"]
        command = orchestrator["command"]
        if not command or not all(isinstance(part, str) for part in command):
            raise ValueError(f"{path}: orchestrator.command is a non-empty list of strings")
        cwd = (path.parent / orchestrator["cwd"]).resolve()
        if not cwd.is_dir():
            raise ValueError(f"{path}: orchestrator.cwd is not a folder: {cwd}")
        # The program is resolved here, not by the launch: Windows looks for a
        # relative program in the trainer's own working directory, not in cwd.
        program = (cwd / command[0]).resolve()
        if not program.is_file():
            raise ValueError(
                f"{path}: the orchestrator is not built: {program} "
                "(cargo build --release -p orchestrator)"
            )
        orin_port = port(path, orchestrator, "orin_port")
        unity_port = port(path, orchestrator, "unity_port")
        if orin_port == unity_port:
            raise ValueError(f"{path}: orin_port and unity_port are both {orin_port}; each socket needs its own port")
        seed = orchestrator["seed"]
        # The orchestrator reads it as a u64.
        if not isinstance(seed, int) or isinstance(seed, bool) or not 0 <= seed < 2**64:
            raise ValueError(f"{path}: orchestrator.seed is a whole number, 0 to 2**64 - 1, got {seed!r}")
        rnd_delay_min = milliseconds(path, orchestrator, "rnd_delay_min")
        rnd_delay_max = milliseconds(path, orchestrator, "rnd_delay_max")
        if rnd_delay_min > rnd_delay_max:
            raise ValueError(f"{path}: rnd_delay_min {rnd_delay_min} is above rnd_delay_max {rnd_delay_max}")
        raw_mjcf = raw["mjcf"]
        mjcf = Mjcf(
            model=(cwd / raw_mjcf["model"]).resolve(),
            cart_joint=raw_mjcf["cart_joint"],
            pole_joint=raw_mjcf["pole_joint"],
            actuator=raw_mjcf["actuator"],
        )
        if not mjcf.model.is_file():
            raise ValueError(f"{path}: mjcf.model is not a file: {mjcf.model}")
        inserted = [
            "--orin-port", str(orin_port),
            "--unity-port", str(unity_port),
            "--seed", str(seed),
            "--rnd-delay-min", str(rnd_delay_min),
            "--rnd-delay-max", str(rnd_delay_max),
        ]
        return Settings(
            # The inserted arguments go first: --orin takes every argument after it.
            orchestrator_command=[str(program), *inserted, *mjcf.arguments(), *command[1:]],
            orchestrator_cwd=cwd,
            orin_port=orin_port,
            unity_port=unity_port,
            seed=seed,
            rnd_delay_min=rnd_delay_min,
            rnd_delay_max=rnd_delay_max,
            mjcf=mjcf,
        )


def port(path: Path, table: dict, key: str) -> int:
    value = table[key]
    # bool is an int in Python; a port of `true` is a typo, not 1.
    if not isinstance(value, int) or isinstance(value, bool) or not 1 <= value <= 65535:
        raise ValueError(f"{path}: orchestrator.{key} is a port, 1-65535, got {value!r}")
    return value


def milliseconds(path: Path, table: dict, key: str) -> float:
    value = table[key]
    # TOML gives 8 as an int and 8.5 as a float; both are milliseconds.
    if not isinstance(value, (int, float)) or isinstance(value, bool) or not 0 <= value < float("inf"):
        raise ValueError(f"{path}: orchestrator.{key} is milliseconds, a finite number >= 0, got {value!r}")
    return value
