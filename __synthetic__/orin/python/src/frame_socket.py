"""The frame socket: the Orin's connection to its camera, the Unity player.

The Orin drives its camera - it sends the trigger and gets the frame back
(__documentation__/synthetic-loop-dia.md, 12-13) - over a WebSocket the
player serves: the player's StateServer listens on the port the orchestrator
launched it with (-state-server), and the Orin connects to it (7).

The orchestrator starts the player before the Orin, but a starting player
takes a moment to listen. So a connection the player refuses is tried again
every connect_retry_millis, until player_start_timeout_secs have passed since
the first try ([frame_socket] in settings.toml): the rule and the values the
dataset generator uses to connect to the same player
(__synthetic__/generator/rust/generator.toml).
"""

import time
import tomllib
from dataclasses import dataclass
from pathlib import Path

from websockets.sync.client import ClientConnection, connect


@dataclass(frozen=True)
class FrameSocketSettings:
    connect_retry_millis: int  # the wait between two tries, ms
    player_start_timeout_secs: int  # how long the player may take to listen, s

    @staticmethod
    def load(path: Path) -> "FrameSocketSettings":
        """[frame_socket] of the Orin's settings file."""
        with open(path, "rb") as f:
            raw = tomllib.load(f)["frame_socket"]
        settings = FrameSocketSettings(
            connect_retry_millis=int(raw["connect_retry_millis"]),
            player_start_timeout_secs=int(raw["player_start_timeout_secs"]),
        )
        if settings.connect_retry_millis <= 0 or settings.player_start_timeout_secs <= 0:
            raise ValueError(f"{path}: [frame_socket]'s values must be above 0, got {settings}")
        return settings


def connect_to_player(url: str, settings: FrameSocketSettings) -> ClientConnection:
    """Connects to the player's frame socket at `url` (7), trying again while the
    player is not yet listening. A refused connection is an OSError; any other
    failure, such as a server that is not a WebSocket, ends the tries at once."""
    deadline = time.monotonic() + settings.player_start_timeout_secs
    while True:
        try:
            # No message-size limit, as on the control socket: a 728x544 frame
            # is ~21 KB, and a contract breach is the detector's to report
            # (a failed frame), not the transport's to drop.
            return connect(url, max_size=None)
        except OSError as error:
            if time.monotonic() >= deadline:
                raise ConnectionError(
                    f"the player at {url} did not accept a connection within "
                    f"{settings.player_start_timeout_secs} s: {error}"
                ) from error
            time.sleep(settings.connect_retry_millis / 1000)
