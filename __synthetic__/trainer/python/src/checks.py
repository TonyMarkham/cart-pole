"""Reading one value from a toml table, with a message naming its key.

Every part checks its own table (rl-rewrite-plan.md, 3.2), so they share
these readers; each refusal names the table and key as the toml spells
them, e.g. `reward.tip.width`.
"""

import math


def table(parent: dict, key: str, where: str) -> dict:
    value = parent.get(key)
    if not isinstance(value, dict):
        raise ValueError(f"{where}{key}: a table is required, got {value!r}")
    return value


def present(t: dict, key: str, where: str):
    if key not in t:
        raise ValueError(f"{where}{key}: required, missing")
    return t[key]


def whole(t: dict, key: str, where: str, minimum: int | None = None, maximum: int | None = None) -> int:
    value = present(t, key, where)
    # bool is an int in Python; `true` is a typo, not 1.
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{where}{key}: a whole number, got {value!r}")
    if minimum is not None and value < minimum or maximum is not None and value > maximum:
        raise ValueError(f"{where}{key}: from {minimum} to {maximum}, got {value}")
    return value


def number(t: dict, key: str, where: str, low: float | None = None, high: float | None = None,
           low_open: bool = False) -> float:
    """A finite number in [low, high], or (low, high] when low_open."""
    value = present(t, key, where)
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value):
        raise ValueError(f"{where}{key}: a finite number, got {value!r}")
    if low is not None and (value <= low if low_open else value < low) or high is not None and value > high:
        bracket = "(" if low_open else "["
        raise ValueError(f"{where}{key}: in {bracket}{low}, {high}], got {value}")
    return float(value)


def name(t: dict, key: str, where: str) -> str:
    value = present(t, key, where)
    if not isinstance(value, str) or not value.isidentifier():
        raise ValueError(f"{where}{key}: a plugin name (a Python identifier), got {value!r}")
    return value


def only(t: dict, keys: set[str], where: str) -> None:
    """A key the part does not read is a typo, not a setting."""
    extra = set(t) - keys
    if extra:
        raise ValueError(f"{where}: unknown key(s) {sorted(extra)}; the keys are {sorted(keys)}")
