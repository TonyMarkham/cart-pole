# The Orin's Python

The Orin has no monitor, so the only way to see what its camera sees is to send
a picture to a browser. `src/capture_push.py` takes one picture with the Global
Shutter camera and pushes it to every open viewer page. It's the start of the
Python that will also measure the calibration's stops, none of which is written
yet.

It reads [settings.toml](settings.toml), and runs on the Orin only, because the
capture uses NVIDIA's camera software.

**There's nothing to build.** `uv` installs what the script needs into a `.venv`
folder beside it the first time it runs, leaving the Orin's own Python alone.

## What it needs on the Orin

- `uv`, which is at `~/.local/bin/uv` on `robo-orin`.
- `gst-launch-1.0` and NVIDIA's camera software, which come with JetPack, the
  Orin's operating system.
- **The webserver running, and `image-push` built,** both in `~/orin-cam`
  ([their README](../rust/README.md)). `settings.toml`'s `[push]`
  table names them.

## Move it to the Orin

On the PC, in Git Bash, from this folder, pack it. The PC's `.venv` stays behind,
because it wouldn't work on the Orin:

```bash
tar --exclude=./.venv -czf ../../../target/live-orin-python.tgz .
```

Move `target/live-orin-python.tgz` from the repo's root to the Orin's home folder
([how](../../README.md)). On the Orin, unpack it. Files of the same name are
replaced and nothing is deleted:

```bash
mkdir -p ~/live-orin-python && tar -xzf ~/live-orin-python.tgz -C ~/live-orin-python
```

This is where it is on `robo-orin`, since 2026-10-31. An older temporary copy in
`/tmp/live-orin-python` is no longer needed.

## Run it

On the Orin, with a viewer open at <http://192.168.0.215:8080/viewer/>:

```bash
cd ~/live-orin-python && uv run python src/capture_push.py
```

It takes about 8 s, nearly all of it writing the frames. The last lines it prints
are these, and the picture appears in the viewer:

```text
Pushed /tmp/orin-capture.png
pushed to http://localhost:8080
```
