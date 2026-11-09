"""The trainer's settings: settings.toml (the run) and the gym file it names.

settings.toml holds what the orchestrator is launched with
(synthetic-loop-dia.md, 1): the command, the ports, the instances' grid and
the screen it fills, the seed, the random timings' bands and the loss
(synthetic-loop-dia.md, Timing), the calibration stop gap, [mjcf] and
[drive]. The gym file (settings.toml's [trainer] gym) holds a whole
variant of the learning: [env], the four plugins ([observation], [action],
[reward], [end]), [train] and [sac]. One value of it reaches the
orchestrator: [env] max_episode_steps, which sets how long each episode's
schedule is. This
module only routes: each plugin checks its own
table (plugins.py), and [train] and [sac] are learning.py's. Relative
paths resolve against the file that holds them, so the trainer runs the
same from any working directory.
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path

import checks
import plugins
from contracts import RunValues
from learning import Sac, Train
from plugins import Part, RunFacts
from timings import Timings

# __synthetic__/trainer/python/settings.toml, beside pyproject.toml.
DEFAULT_PATH = Path(__file__).resolve().parent.parent / "settings.toml"

SEED_LIMIT = 2**64  # the orchestrator reads the seed as a u64
INSTANCES_MAX = 12  # the orchestrator's limit on rows x columns (synthetic-loop-dia.md, Training instances)

SITE_KEYS = ("left_site", "right_site", "pivot_site", "end_site", "target_site", "dynamic_target_site",
             "cart_site", "cart_target_site", "left_rail_site", "left_cart_site", "right_rail_site",
             "right_cart_site")


@dataclass(frozen=True)
class Mjcf:
    model: Path  # the MJCF the run loads (absolute)
    cart_joint: str  # as in the model's <joint name>; also a wire key
    pole_joint: str
    actuator: str
    sites: dict  # the [mjcf] *_site key -> the model's <site name>

    def arguments(self) -> list[str]:
        sites = [arg for key in SITE_KEYS for arg in ("--" + key.replace("_", "-"), self.sites[key])]
        return ["--model", str(self.model), "--cart-joint", self.cart_joint, "--pole-joint", self.pole_joint,
                "--actuator", self.actuator, *sites]


@dataclass(frozen=True)
class Drive:
    """[drive]: the figures between a pulse rate and the cart's speed, which
    the MJCF does not hold (metres per pulse = belt_um_per_rev /
    (full_steps_per_rev x microsteps)), and the jump limit the drive's ramp
    holds (synthetic-loop-dia.md, The acceleration ceiling)."""

    full_steps_per_rev: int
    microsteps: int
    belt_um_per_rev: int
    jump_limit: int  # m, pulses/s^2

    def arguments(self) -> list[str]:
        return ["--full-steps-per-rev", str(self.full_steps_per_rev), "--microsteps", str(self.microsteps),
                "--belt-um-per-rev", str(self.belt_um_per_rev), "--jump-limit", str(self.jump_limit)]


@dataclass(frozen=True)
class Settings:
    path: Path  # settings.toml itself (absolute)
    program: Path  # the orchestrator binary (absolute)
    arguments: tuple[str, ...]  # the command's arguments after the program, as written
    orchestrator_cwd: Path
    orin_port: int  # instance 0's control socket; instance k uses + 2k
    unity_port: int
    instance_rows: int
    instance_columns: int
    screen_width: int  # pixels: the area the grid of windows fills
    screen_height: int
    seed: int
    timings: Timings  # the random timings' bands and the loss
    calibration_stop_gap: float  # m: the calibration's end stops, short of each base
    mjcf: Mjcf
    drive: Drive
    gym: Path  # [trainer] gym (absolute)
    measures: Path | None  # [trainer] measures (absolute), the measures' thresholds

    @property
    def instances(self) -> int:
        return self.instance_rows * self.instance_columns

    def grid(self, instances: int) -> tuple[int, int]:
        """The rows and columns of a launch of `instances`: the settings' grid
        for the whole group, 1 x 1 for env_check's one instance."""
        if instances == self.instances:
            return self.instance_rows, self.instance_columns
        if instances == 1:
            return 1, 1
        raise ValueError(f"a launch runs {self.instances} instances or 1, got {instances}")

    def facts(self, values: RunValues) -> RunFacts:
        """The run's facts: the longest gap between captures from the bands
        here, the rest from the orchestrator's answer (the link's ask())."""
        return RunFacts(capture_gap_max_ms=self.timings.longest_capture_gap_ms(),
                        cart_top_speed=values.cart_top_speed,
                        top_pulse_rate=values.top_pulse_rate, timestep=values.timestep,
                        pole_mass=values.pole_mass, pole_com_distance=values.pole_com_distance,
                        pole_inertia=values.pole_inertia, gravity=values.gravity)

    def command(self, seed: int, instances: int, out: Path | None = None) -> list[str]:
        """The orchestrator's launch command: the inserted flags straight after
        the program (--orin takes every argument after it), the random
        timings and the gym file's max_episode_steps among them, then [mjcf]
        and [drive], then the command as written, with `out` in place of its
        --out."""
        if isinstance(seed, bool) or not isinstance(seed, int) or not 0 <= seed < SEED_LIMIT:
            raise ValueError(f"a run's seed is a whole number, 0 to 2**64 - 1, got {seed!r}")
        rows, columns = self.grid(instances)
        inserted = ["--instance-rows", str(rows), "--instance-columns", str(columns),
                    "--screen-width", str(self.screen_width), "--screen-height", str(self.screen_height),
                    "--orin-port", str(self.orin_port),
                    "--unity-port", str(self.unity_port), "--seed", str(seed),
                    *self.timings.arguments(),
                    "--max-episode-steps", str(read_max_episode_steps(self.gym)),
                    "--calibration-stop-gap", str(self.calibration_stop_gap)]
        arguments = list(self.arguments)
        if out is not None:
            own = arguments.index("--orin") if "--orin" in arguments else len(arguments)
            if "--out" not in arguments[:own]:
                raise ValueError("orchestrator.command has no --out to replace")
            arguments[arguments.index("--out") + 1] = str(out)
        return [str(self.program), *inserted, *self.mjcf.arguments(), *self.drive.arguments(), *arguments]

    @staticmethod
    def load(path: Path = DEFAULT_PATH) -> "Settings":
        path = path.resolve()
        with open(path, "rb") as f:
            raw = tomllib.load(f)
        o = checks.table(raw, "orchestrator", "")
        command = checks.present(o, "command", "orchestrator.")
        if not isinstance(command, list) or not command or not all(isinstance(p, str) for p in command):
            raise ValueError(f"{path}: orchestrator.command: a non-empty list of strings")
        cwd = (path.parent / checks.present(o, "cwd", "orchestrator.")).resolve()
        if not cwd.is_dir():
            raise ValueError(f"{path}: orchestrator.cwd is not a folder: {cwd}")
        # Resolved here: Windows looks for a relative program in the
        # trainer's own working directory, not in cwd.
        program = (cwd / command[0]).resolve()
        orin_port = checks.whole(o, "orin_port", "orchestrator.", 1, 65535)
        unity_port = checks.whole(o, "unity_port", "orchestrator.", 1, 65535)
        rows = checks.whole(o, "instance_rows", "orchestrator.", 1, INSTANCES_MAX)
        columns = checks.whole(o, "instance_columns", "orchestrator.", 1, INSTANCES_MAX)
        instances = rows * columns
        if instances > INSTANCES_MAX:
            raise ValueError(f"{path}: instance_rows {rows} x instance_columns {columns} is {instances} "
                             f"instances; at most {INSTANCES_MAX}")
        screen_width = checks.whole(o, "screen_width", "orchestrator.", 1)
        screen_height = checks.whole(o, "screen_height", "orchestrator.", 1)
        group = [p + 2 * k for k in range(instances) for p in (orin_port, unity_port)]
        if len(set(group)) != len(group) or max(group) > 65535:
            raise ValueError(f"{path}: with {instances} instances the ports are {group}; "
                             "each must be its own and at most 65535")
        seed = checks.whole(o, "seed", "orchestrator.", 0, SEED_LIMIT - 1)
        try:
            timings = Timings.from_table(o)
        except ValueError as error:
            raise ValueError(f"{path}: {error}") from None
        calibration_stop_gap = checks.number(o, "calibration_stop_gap", "orchestrator.", 0.0)
        m = checks.table(raw, "mjcf", "")
        mjcf = Mjcf(
            model=(cwd / checks.present(m, "model", "mjcf.")).resolve(),
            cart_joint=checks.present(m, "cart_joint", "mjcf."),
            pole_joint=checks.present(m, "pole_joint", "mjcf."),
            actuator=checks.present(m, "actuator", "mjcf."),
            sites={key: checks.present(m, key, "mjcf.") for key in SITE_KEYS},
        )
        d = checks.table(raw, "drive", "")
        keys = ("full_steps_per_rev", "microsteps", "belt_um_per_rev", "jump_limit")
        checks.only(d, set(keys), "drive")
        drive = Drive(*(checks.whole(d, key, "drive.", 1) for key in keys))
        t = checks.table(raw, "trainer", "")
        return Settings(
            path=path, program=program, arguments=tuple(command[1:]), orchestrator_cwd=cwd,
            orin_port=orin_port, unity_port=unity_port, instance_rows=rows, instance_columns=columns,
            screen_width=screen_width, screen_height=screen_height, seed=seed,
            timings=timings, calibration_stop_gap=calibration_stop_gap, mjcf=mjcf, drive=drive,
            gym=named_file(path, t, "gym"),
            measures=named_file(path, t, "measures") if "measures" in t else None,
        )

    def require_built(self) -> None:
        """The checks a launch needs and a test does not: the binary and the model exist."""
        if not self.program.is_file():
            raise ValueError(f"{self.path}: the orchestrator is not built: {self.program} "
                             "(cargo build --release -p orchestrator)")
        if not self.mjcf.model.is_file():
            raise ValueError(f"{self.path}: mjcf.model is not a file: {self.mjcf.model}")


def named_file(path: Path, trainer: dict, key: str) -> Path:
    file = (path.parent / checks.present(trainer, key, "trainer.")).resolve()
    if not file.is_file():
        raise ValueError(f"{path}: trainer.{key} is not a file: {file}")
    return file


GYM_TABLES = {"env", "observation", "action", "reward", "end", "train", "sac"}


def read_train(path: Path) -> Train:
    """[train] alone from the gym file. It needs none of the run's values,
    and a training run needs it before the launch: the run folder (the
    orchestrator's log and --out) and the first episode's seed come from
    it. Gym.load reads and checks it again with the rest of the file."""
    path = path.resolve()
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    try:
        return Train.from_table(checks.table(raw, "train", ""), path.parent)
    except ValueError as error:
        raise ValueError(f"{path}: {error}") from None


def read_max_episode_steps(path: Path) -> int:
    """[env] max_episode_steps alone from the gym file. It reaches the
    orchestrator on its command line, as --max-episode-steps, which sizes
    each episode's schedule (synthetic-loop-dia.md, The schedule), before the
    run's values Gym.load needs exist. Gym.load reads and checks it again
    with the rest of the file."""
    path = path.resolve()
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    try:
        return checks.whole(checks.table(raw, "env", ""), "max_episode_steps", "env.", 1)
    except ValueError as error:
        raise ValueError(f"{path}: {error}") from None


@dataclass(frozen=True)
class Gym:
    path: Path
    text: str  # the file as read, for the run's manifest
    max_episode_steps: int  # [env]: the time limit, in steps
    observation: Part
    action: Part
    reward: Part
    endings: tuple[Part, ...]
    train: Train
    sac: Sac
    facts: RunFacts  # what the plugins were built with: the environment reads the run's values here

    @staticmethod
    def load(path: Path, facts: RunFacts) -> "Gym":
        path = path.resolve()
        text = path.read_text(encoding="utf-8")
        raw = tomllib.loads(text)
        checks.only(raw, GYM_TABLES, f"{path}")
        try:
            env = checks.table(raw, "env", "")
            checks.only(env, {"max_episode_steps"}, "env")
            steps = checks.whole(env, "max_episode_steps", "env.", 1)
            observation = chosen(raw, "observation", "plugin", "observation", facts)
            action = chosen(raw, "action", "plugin", "action", facts)
            reward = chosen(raw, "reward", "scheme", "reward", facts)
            end = checks.table(raw, "end", "")
            listed = checks.present(end, "plugins", "end.")
            if not isinstance(listed, list) or not all(isinstance(n, str) for n in listed) \
                    or len(set(listed)) != len(listed):
                raise ValueError(f"end.plugins: a list of distinct plugin names, got {listed!r}")
            scalars(end, {"plugins"}, "end")
            endings = tuple(plugins.load("end", n, end, "end", facts) for n in listed)
            train = Train.from_table(checks.table(raw, "train", ""), path.parent)
            sac = Sac.from_table(checks.table(raw, "sac", ""))
        except ValueError as error:
            raise ValueError(f"{path}: {error}") from None
        return Gym(path, text, steps, observation, action, reward, endings, train, sac, facts)

    def parts(self) -> list[Part]:
        return [self.observation, self.action, self.reward, *self.endings]


def chosen(raw: dict, table_key: str, name_key: str, role: str, facts: RunFacts) -> Part:
    """[observation] plugin = "x" with its table [observation.x], and the
    same for [action]. Tables for other plugins of the role may sit beside
    it, so variants can be kept side by side. [reward] scheme = "x" takes
    the factor tables x names (plugins.py: TABLES), [reward.centered] and
    so on, and ignores any other."""
    t = checks.table(raw, table_key, "")
    scalars(t, {name_key}, table_key)
    return plugins.load(role, checks.name(t, name_key, f"{table_key}."), t, table_key, facts)


def scalars(t: dict, keys: set[str], where: str) -> None:
    """Besides `keys`, a role's table holds only plugin tables."""
    extra = [k for k, v in t.items() if k not in keys and not isinstance(v, dict)]
    if extra:
        raise ValueError(f"{where}: unknown key(s) {sorted(extra)}; besides {sorted(keys)} it holds plugin tables")
