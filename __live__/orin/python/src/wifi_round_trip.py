"""Measures how long a message takes to go from the Orin to the Arduino over WiFi and
back. The delay D, from a frame's exposure to its command reaching the board, includes
the WiFi hop, which the Orin's clock cannot time on its own (__documentation__/real-loop-dia.md,
Measuring D). A reply from the board can.

The board already answers one request without moving anything: "pins", with its
pins' levels (the firmware's `PINS_REQUEST`). This sends it `count` times, `gap_ms`
apart, and times each answer with the Orin's clock. The round trip is:

    the request's WiFi hop to the board's WiFi chip
  + the chip pushing it to the board's processor
  + the processor's answer: 5 exchanges with the WiFi chip at 115,200 baud (asking
    who sent it, its address and port, then starting, filling and sending the
    answer, `answer_levels` in the firmware's wifi.rs)
  + the answer's WiFi hop back

A command in Run is one way and gets no answer, so its hop to the board is shorter
than this round trip: the round trip is an upper bound for it.

An answer that takes longer than `timeout_ms` is counted lost, and any late answer
is thrown away before the next request, so it is never taken for that one's.

Close the /arduino page first: it asks for the pins every second, and the board
answers one request at a time.

The settings are [round_trip] in settings.toml, beside this folder.

usage (from __live__/orin/python): .venv/bin/python src/wifi_round_trip.py
"""

import socket
import sys
import time
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python 3.10, the Orin's system Python
    import tomli as tomllib

FOLDER = Path(__file__).resolve().parent.parent

# The firmware's request and the start of its answer, {"d2":0,...}: PINS_REQUEST
# in the firmware's constants.rs and the webserver's.
PINS_REQUEST = b"pins"
ANSWER_START = b"{"

# Room for the answer: PINS_ANSWER_MAX in the firmware's constants.rs.
ANSWER_MAX = 64


def drain(sock: socket.socket, timeout_s: float) -> int:
    """Throws away every answer already waiting, and says how many there were. The
    wait for an answer is then timeout_s again: setblocking(True) would wait forever,
    and one lost answer would hang the run."""
    sock.settimeout(0.0)
    thrown = 0
    try:
        while True:
            sock.recv(ANSWER_MAX)
            thrown += 1
    except BlockingIOError:
        pass
    finally:
        sock.settimeout(timeout_s)
    return thrown


def percentile(sorted_ms: list[float], share: float) -> float:
    """The time `share` of the answers came within: the nearest-rank percentile."""
    return sorted_ms[min(len(sorted_ms) - 1, max(0, round(share * len(sorted_ms)) - 1))]


def main() -> int:
    table = tomllib.loads((FOLDER / "settings.toml").read_text())["round_trip"]
    target = (table["address"], int(table["port"]))
    count, gap_s, timeout_s = int(table["count"]), table["gap_ms"] / 1_000, table["timeout_ms"] / 1_000
    out = FOLDER / table["out"]

    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.connect(target)  # only the board's datagrams are received
    rows, late = [], 0
    print(f"wifi_round_trip: {count} requests to {target[0]}:{target[1]}, {table['gap_ms']} ms apart", flush=True)
    for n in range(count):
        late += drain(sock, timeout_s)
        start = time.perf_counter_ns()
        sock.send(PINS_REQUEST)
        try:
            answer = sock.recv(ANSWER_MAX)
            ms = (time.perf_counter_ns() - start) / 1e6
            rows.append((n, f"{ms:.3f}", "" if answer.startswith(ANSWER_START) else answer.decode(errors="replace")))
        except socket.timeout:
            rows.append((n, "", "lost"))
        except ConnectionRefusedError:
            # The Orin was told nothing listens at the board's port.
            rows.append((n, "", "refused"))
        time.sleep(gap_s)
    late += drain(sock, timeout_s)

    out.write_text("request,round_trip_ms,note\n" + "".join(f"{n},{ms},{note}\n" for n, ms, note in rows))
    times = sorted(float(ms) for _, ms, note in rows if ms and not note)
    lost = sum(1 for _, ms, _ in rows if not ms)
    odd = sum(1 for _, ms, note in rows if ms and note)
    print(f"answered {len(times)}, lost {lost}, late and thrown away {late}, unexpected answers {odd}")
    if times:
        print(f"round trip, ms: fastest {times[0]:.2f}, median {percentile(times, 0.5):.2f}, "
              f"90% within {percentile(times, 0.9):.2f}, 99% within {percentile(times, 0.99):.2f}, "
              f"slowest {times[-1]:.2f}")
    print(f"each request: {out}")
    return 0 if times else 1


if __name__ == "__main__":
    sys.exit(main())
