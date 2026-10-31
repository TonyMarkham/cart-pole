"""The entropy coefficient at each saved snapshot: SAC's automatically tuned
weight on the bonus for staying random (trainer-observations.md,
Exploration: how random it stays). Read from each model_<steps>.zip's
`log_ent_coef`; the coefficient is its exp.
"""

import math

import checks
from run_folder import RunFolder
from stable_baselines3.common.save_util import load_from_zip_file


def snapshots(folder: RunFolder) -> list[tuple[int, object]]:
    found = [(int(p.stem.split("_", 1)[1]), p) for p in folder.path.glob("model_*.zip")]
    return sorted(found)


def run(folder: RunFolder, table: dict) -> None:
    checks.only(table, set(), "entropy")
    print("| Steps | Entropy coefficient |")
    print("|---|---|")
    for steps, path in snapshots(folder):
        _, _, variables = load_from_zip_file(path, device="cpu")
        print(f"| {steps:,d} | {math.exp(variables['log_ent_coef'].item()):.5f} |")
