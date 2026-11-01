"""The frame socket: where the Unity player, the Orin's camera, connects.

The Orin drives its camera - it sends the trigger and gets the frame back
(synthetic-loop-dia.md, 12-13) - so it serves this socket and Unity is the
client (6-7). It takes exactly one camera: a second connection is refused.

websockets' sync server runs each connection's handler in its own thread
and closes the connection when the handler returns, so the handler hands
the connection to the main thread and then waits until close() - the
connection lives as long as the run.
"""

import queue
import threading

from websockets.sync.server import ServerConnection, serve


class FrameSocket:
    def __init__(self, host: str, port: int):
        self._camera: queue.Queue[ServerConnection] = queue.Queue(maxsize=1)
        self._closing = threading.Event()
        # No message-size limit, as on the control socket: a 728x544 frame
        # is ~21 KB, and a contract breach is the detector's to report
        # (a failed frame), not the transport's to drop.
        self._server = serve(self._handler, host, port, max_size=None)
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)
        self._thread.start()

    def _handler(self, connection: ServerConnection) -> None:
        try:
            self._camera.put_nowait(connection)
        except queue.Full:
            connection.close(reason="the synthetic Orin takes one camera")
            return
        self._closing.wait()

    def wait_for_camera(self) -> ServerConnection:
        """Blocks until Unity has connected (7)."""
        return self._camera.get()

    def close(self) -> None:
        """Closes the camera's connection and the listener (23)."""
        self._closing.set()
        for connection in list(self._server.connections):
            connection.close()
        self._server.shutdown()
        self._thread.join()
