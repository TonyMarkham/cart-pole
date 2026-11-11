"""Writes a trained policy as one numpy file the Orin can run without PyTorch
(__documentation__/real-loop-dia.md, The policy on the Orin).

The rig needs only SAC's actor, the network that picks the command, and only
its best guess: a = tanh(mean), with no randomness. The actor's best guess is
two hidden layers and one output:

    h1   = relu(W1 x + b1)      x: the 36 inputs; W1 256 x 36
    h2   = relu(W2 h1 + b2)     W2 256 x 256
    mean = W3 h2 + b3           W3 1 x 256
    a    = tanh(mean)

relu(v) is max(v, 0), each number on its own. These are Stable-Baselines3's
`actor.latent_pi` (Linear, ReLU, Linear, ReLU) and `actor.mu`; its
`actor.log_std`, the width of the bell curve training drew from, is left out
because the rig never draws. a is in [-1, 1], and the command is
round(a x top_pulse_rate) pulses/s (`actions/linear_rate.py`).

The file also holds the run's values the inputs and the command must be built
with (top_pulse_rate, age_unit_ms and past_commands), so they
cannot drift from what the policy learned, and `check_samples` inputs from the
run's replay buffer with Stable-Baselines3's own answer for each. Whoever runs
the file can work those out and compare, with numpy alone.

Before writing, every input in the replay buffer is worked out both ways, by
Stable-Baselines3 (`predict(observation, deterministic=True)`) and by the
formula above, and the file is refused if any answer differs by more than
TOLERANCE.

usage (from the repo root):
  uv run --project __synthetic__/trainer/python python __synthetic__/trainer/python/src/export_policy.py [RUN_FOLDER] [--model FILE] [--out FILE] [--settings FILE]
Without RUN_FOLDER, the most recent run in [train] runs_dir that has a save;
without --model, its latest.zip; without --out, policy.npz in the run folder.
Its values (check_samples, seed) are the [export_policy] table in the file
settings.toml's [trainer] measures names.
"""

import argparse
import json
import sys
import tomllib
from pathlib import Path

import numpy as np
from stable_baselines3 import SAC
from stable_baselines3.common.save_util import load_from_pkl

import checks
from contracts import FLOATS
from run_folder import RunFolder
from settings import DEFAULT_PATH, Settings, read_train

# The actor works in float32, which keeps about 7 significant digits; a sum of
# 256 products rounds differently from numpy's float64 by about 1e-6 at most.
# A wrong layer, order or activation is off by 0.01 or more.
TOLERANCE = 1e-5

# Inputs worked out per batch, so the whole buffer never sits in memory three times.
BATCH = 100_000


def best_guess(weights: dict[str, np.ndarray], inputs: np.ndarray) -> np.ndarray:
    """The formula in the module's docstring, for a batch of inputs, one per row."""
    x = inputs.astype(np.float64)
    h1 = np.maximum(x @ weights["hidden_1_weight"].T.astype(np.float64) + weights["hidden_1_bias"], 0.0)
    h2 = np.maximum(h1 @ weights["hidden_2_weight"].T.astype(np.float64) + weights["hidden_2_bias"], 0.0)
    return np.tanh(h2 @ weights["output_weight"].T.astype(np.float64) + weights["output_bias"])


def actor_weights(model: SAC) -> dict[str, np.ndarray]:
    """The best guess's weights, refused unless the actor is the shape the formula
    expects: two hidden layers, ReLU between them, one output."""
    latent = model.actor.latent_pi
    kinds = [type(layer).__name__ for layer in latent]
    if kinds != ["Linear", "ReLU", "Linear", "ReLU"] or model.actor.mu.out_features != 1:
        raise SystemExit(f"export_policy: the actor is {kinds} then {model.actor.mu}, not two ReLU layers and one output")
    return {
        name: layer.detach().cpu().numpy().astype(np.float32)
        for name, layer in (
            ("hidden_1_weight", latent[0].weight), ("hidden_1_bias", latent[0].bias),
            ("hidden_2_weight", latent[2].weight), ("hidden_2_bias", latent[2].bias),
            ("output_weight", model.actor.mu.weight), ("output_bias", model.actor.mu.bias),
        )
    }


def run_values(manifest: dict) -> dict[str, np.ndarray]:
    """The values the brain builds the inputs and the command with, from the run's
    manifest (__documentation__/real-loop-dia.md, The policy on the Orin)."""
    observation = manifest["plugins"]["observation"]
    if observation["name"] != "pool_and_commands" or manifest["plugins"]["action"]["name"] != "linear_rate":
        raise SystemExit("export_policy: only the pool_and_commands inputs and the linear_rate command are written")
    return {
        "top_pulse_rate": np.int64(manifest["run_values"]["top_pulse_rate"]),
        "age_unit_ms": np.float64(observation["table"]["age_unit_ms"]),
        "past_commands": np.int64(observation["table"]["past_commands"]),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("folder", type=Path, nargs="?")
    parser.add_argument("--model", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--settings", type=Path, default=DEFAULT_PATH)
    args = parser.parse_args()
    settings = Settings.load(args.settings)
    if settings.measures is None:
        raise SystemExit(f"{settings.path}: [trainer] measures names no file")
    with open(settings.measures, "rb") as f:
        table = tomllib.load(f).get("export_policy", {})
    checks.only(table, {"check_samples", "seed"}, "export_policy")
    check_samples = checks.whole(table, "check_samples", "export_policy.", 1)
    seed = checks.whole(table, "seed", "export_policy.", 0)

    folder = RunFolder(args.folder.resolve()) if args.folder is not None else RunFolder.latest(read_train(settings.gym).runs_dir)
    model_path = args.model.resolve() if args.model is not None else folder.model
    manifest_path = model_path.with_suffix(".json")
    out = args.out.resolve() if args.out is not None else folder.path / "policy.npz"
    print(f"export_policy: {model_path}", flush=True)
    manifest = json.loads(manifest_path.read_text())
    model = SAC.load(model_path, device="cpu")
    weights = actor_weights(model)
    inputs = FLOATS + manifest["plugins"]["observation"]["table"]["past_commands"]
    if weights["hidden_1_weight"].shape[1] != inputs:
        raise SystemExit(f"export_policy: the actor reads {weights['hidden_1_weight'].shape[1]} inputs, the run's manifest {inputs}")

    buffer = load_from_pkl(folder.buffer)
    rows = buffer.buffer_size if buffer.full else buffer.pos
    observations = buffer.observations[:rows].reshape(rows * buffer.n_envs, -1)
    worst = 0.0
    for start in range(0, len(observations), BATCH):
        batch = observations[start:start + BATCH]
        theirs, _ = model.predict(batch, deterministic=True)
        worst = max(worst, float(np.abs(best_guess(weights, batch) - theirs.astype(np.float64)).max()))
    print(f"export_policy: {len(observations):,d} buffer inputs, worst difference {worst:.2e}", flush=True)
    if worst > TOLERANCE:
        raise SystemExit(f"export_policy: the formula differs from Stable-Baselines3 by {worst:.2e}, over {TOLERANCE:.0e}")

    chosen = np.sort(np.random.default_rng(seed).choice(len(observations), size=min(check_samples, len(observations)),
                                                        replace=False))
    check_inputs = observations[chosen].astype(np.float32)
    check_actions, _ = model.predict(check_inputs, deterministic=True)
    out.parent.mkdir(parents=True, exist_ok=True)
    temporary = out.with_name(out.name + ".tmp.npz")
    np.savez(
        temporary,
        **weights,
        **run_values(manifest),
        check_inputs=check_inputs,
        check_actions=check_actions.astype(np.float32).reshape(-1),
        source=np.str_(f"{folder.path.name}/{model_path.name}, {manifest['steps']:,d} steps, commit {manifest['git']['commit'][:8]}"
                       f"{' with uncommitted changes' if manifest['git']['dirty'] else ''}"),
    )
    temporary.replace(out)
    print(f"export_policy: wrote {out}", flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
