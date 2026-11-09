"""The actor's spread at each saved snapshot: how far the policy strays
from its best guess (trainer-observations.md, *How it was measured*).

`samples` observations are drawn at random (seeded by `seed`) from the
run's latest_buffer.pkl and given to each snapshot's actor; the spread is
exp(log_std) of its bell curve, before the squash into [-1, 1]. The table
shows its median and the median |mean action| beside it.
"""

import numpy as np
import torch
from stable_baselines3 import SAC
from stable_baselines3.common.save_util import load_from_pkl

import checks
from measures.entropy import snapshots
from run_folder import RunFolder


def run(folder: RunFolder, table: dict) -> None:
    checks.only(table, {"samples", "seed"}, "actor_spread")
    samples = checks.whole(table, "samples", "actor_spread.", 1)
    seed = checks.whole(table, "seed", "actor_spread.", 0)
    buffer = load_from_pkl(folder.buffer)
    rows = buffer.buffer_size if buffer.full else buffer.pos
    observations = buffer.observations[:rows].reshape(rows * buffer.n_envs, -1)
    chosen = np.random.default_rng(seed).choice(len(observations), size=min(samples, len(observations)),
                                                replace=False)
    batch = torch.as_tensor(observations[chosen], dtype=torch.float32)
    print("| Steps | Median spread | Median abs mean |")
    print("|---|---|---|")
    for steps, path in snapshots(folder):
        model = SAC.load(path, device="cpu")
        with torch.no_grad():
            mean, log_std, _ = model.actor.get_action_dist_params(batch)
        print(f"| {steps:,d} | {float(torch.exp(log_std).median()):.4f} | {float(mean.abs().median()):.4f} |")
