"""What the tests share: __synthetic__/trainer/python/src on the import path, and fixture files.

The fixtures are the tests' own: a settings.toml whose orchestrator need
not exist (the tests run on the fake link) and whose [drive] holds a
jump limit (a test value, not the rig's), a small gym file with every
value the plugins require, and the run's values the fake link answers
with. rig_values() is
the exception: MuJoCo's, from the built orchestrator, for loading the real
settings files.
"""

import sys
import tempfile
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "src"
if str(SRC) not in sys.path:
    sys.path.insert(0, str(SRC))

SETTINGS = """
[trainer]
gym = "gym.toml"
measures = "measures.toml"

[orchestrator]
cwd = "."
command = ["orchestrator.exe", "--out", "out", "--orin", "uv"]
orin_port = 9000
unity_port = 9001
instance_rows = 1
instance_columns = {instances}
screen_width = 3840
screen_height = 2112
seed = 0
rnd_delay_min = 8
rnd_delay_max = 15
calibration_stop_gap = 0.005

[mjcf]
model = "model.xml"
cart_joint = "cart"
pole_joint = "pole"
actuator = "actuator"
left_site = "l"
right_site = "r"
pivot_site = "p"
end_site = "e"
target_site = "t"
dynamic_target_site = "dt"
cart_site = "c"
cart_target_site = "ct"
left_rail_site = "lr"
left_cart_site = "lc"
right_rail_site = "rr"
right_cart_site = "rc"

[drive]
full_steps_per_rev = 200
microsteps = 32
belt_um_per_rev = 96000
jump_limit = 1000000
"""

GYM = """
[env]
command_rate_min = 20.0
command_rate_max = 30.0
max_episode_steps = {max_episode_steps}

[observation]
plugin = "pool_and_commands"

[observation.pool_and_commands]
past_commands = {past_commands}
age_unit_ms = 1000.0

[action]
plugin = "linear_rate"

[reward]
scheme = "dm_control_cartpole"

[reward.small_control]
margin = 1.0
floor = 0.8

[reward.small_velocity]
margin = 5.0
at_margin = 0.1
floor = 0.5

[reward.centered]
margin = {centered_margin}
at_margin = 0.1
floor = 0.5

[end]
plugins = ["strike_gap"]

[end.strike_gap]
buffer = {buffer}

[train]
total_steps = {total_steps}
save_every = {save_every}
runs_dir = "runs"

[sac]
device = "cpu"
learning_starts = 50
buffer_size = 10000
batch_size = 32
learning_rate = 0.0003
gamma = 0.998
tau = 0.005
train_freq = 1
gradient_steps = 1
net_arch = [16, 16]
ent_coef = "auto_0.1"
"""

MEASURES = """
[episodes]
band_steps = 100
upright_degrees = 20.0
spin_turns = 15.0
slow_turns = 10.0
visit_seconds = 0.3

[actor_spread]
samples = 50
seed = 0
"""

# The run's values the fake link answers with (test values, not the rig's,
# which the orchestrator reads from the MJCF and [drive]): the cart top
# speed, metres per pulse (the [drive] above: 96,000 um / (200 x 32)) and
# the timestep. The top pulse rate is 0.9 / 15e-6 = 60,000 pulses/s; the
# strike-gap floor at 20 Hz and rnd_delay_max 15 ms is 0.9 m/s x (52 + 16) ms
# = 61.2 mm.
CART_TOP_SPEED = 0.9  # m/s
METRES_A_PULSE = 15e-6  # m
TIMESTEP = 0.002  # s

GYM_DEFAULTS = {"max_episode_steps": 60, "past_commands": 6, "centered_margin": 2.0, "buffer": 0.062,
                "total_steps": 200, "save_every": 100}


def fake(instances: int, seed: int, cart_top_speed: float = CART_TOP_SPEED, **band):
    """A fake link on the tests' run values (any of them may be swapped)."""
    from link.fake import FakeLink
    return FakeLink(instances, seed, cart_top_speed, METRES_A_PULSE, TIMESTEP, **band)


def run_values():
    """What the fake link answers ask() with."""
    return fake(1, 0).ask()


def rig_values(settings):
    """The run's values the orchestrator answers with for `settings` (a
    loaded settings.toml): MuJoCo's, from the built orchestrator's
    --run-values, which loads the model and prints the same line it answers
    the trainer's request with (synthetic-loop-dia.md, Contracts, the run's
    values). Nothing here works them out."""
    import subprocess
    from link.real import orchestrator_environment, read_run
    settings.require_built()
    command = settings.command(settings.seed, 1)
    command.insert(1, "--run-values")
    result = subprocess.run(command, cwd=settings.orchestrator_cwd, env=orchestrator_environment(),
                            capture_output=True, encoding="utf-8", check=True)
    return read_run(result.stdout)


def facts(rnd_delay_max_ms: float = 15.0, command_rate_min_hz: float = 20.0):
    """The run's facts on the tests' run values."""
    from plugins import RunFacts
    values = run_values()
    return RunFacts(rnd_delay_max_ms, command_rate_min_hz, values.cart_top_speed, values.top_pulse_rate, values.timestep,
                    values.pole_mass, values.pole_com_distance, values.pole_inertia, values.gravity)


class Fixture:
    """A folder holding settings.toml, gym.toml and measures.toml."""

    def __init__(self, instances: int = 2, **gym) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.path = Path(self.temp.name)
        (self.path / "settings.toml").write_text(SETTINGS.format(instances=instances), encoding="utf-8")
        (self.path / "measures.toml").write_text(MEASURES, encoding="utf-8")
        self.write_gym(**gym)

    def write_gym(self, **gym) -> None:
        values = {**GYM_DEFAULTS, **gym}
        (self.path / "gym.toml").write_text(GYM.format(**values), encoding="utf-8")

    def load(self):
        from settings import Gym, Settings
        settings = Settings.load(self.path / "settings.toml")
        return settings, Gym.load(settings.gym, settings.facts(run_values()))

    def close(self) -> None:
        self.temp.cleanup()


def fake_link(settings, instances, folder, steps, cart_top_speed: float = CART_TOP_SPEED):
    """train.train's link argument: the fake rig in place of the orchestrator,
    an opener train() calls with the first seed, then asks."""
    from settings import read_command_rates
    rates = read_command_rates(settings.gym)
    return lambda seed: fake(instances, seed, cart_top_speed, rnd_delay_min=settings.rnd_delay_min,
                             rnd_delay_max=settings.rnd_delay_max, command_rate_min=rates.low_hz,
                             command_rate_max=rates.high_hz)
