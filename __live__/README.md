# `__live__` — what runs on the real rig

The real rig is several computers, and each program has to be built on one
computer, moved to another and started there. This page says which is which;
each part's own README has the exact commands.

**Nothing here runs on the PC.** The repo is the source, and the PC only edits it,
builds what the table below says, and sends it out. `orin/rust` and `rpi/rust` are
pushed whole to their computers, so each must build with nothing else from the repo.

| Part | Folder | Runs on | Built on | Workspace |
|---|---|---|---|---|
| Calibration firmware | `arduino/rust/crates/calibration` | Arduino | PC | the repo's root `Cargo.toml` |
| Viewer page | `orin/wasm/viewer` | a browser | PC | the repo's root `Cargo.toml` |
| Arduino page | `orin/wasm/arduino` | a browser | PC | the repo's root `Cargo.toml` |
| Webserver | `orin/rust/crates/webserver` | Orin | Orin | `orin/rust/Cargo.toml` |
| `image-push` | `orin/rust/crates/image-push` | Orin | Orin | `orin/rust/Cargo.toml` |
| `arduino-manager` | `rpi/rust/crates/arduino-manager` | Pi | Pi | `rpi/rust/Cargo.toml` |
| `capture_push.py` | `orin/python` | Orin | nothing to build | a `uv` project |

A **workspace** is a group of Rust crates built together, with their dependencies'
versions in one `Cargo.toml`. Cargo finds a crate's by walking up the folders from
it. The two pages build on the PC into `orin/rust/pages/`, which is pushed with the
rest of that folder, so the Orin gets their output and never their source.

Each part's README:
[firmware](arduino/rust/crates/calibration/README.md),
[webserver, `image-push` and the pages](orin/rust/README.md),
[manager](rpi/rust/crates/arduino-manager/README.md),
[`capture_push.py`](orin/python/README.md).

The computers on this network, and what's on each since 2026-10-31:

| Computer | Address | Login name | Where | Check it at |
|---|---|---|---|---|
| Pi (`robo-pi`) | `192.168.0.247` | `tony` | `~/arduino-manager` | <http://192.168.0.247:8080/> |
| Orin (`robo-orin`) | `192.168.0.215` | `tony` | `~/orin-cam`, `~/live-orin-python` | <http://192.168.0.215:8080/arduino>, <http://192.168.0.215:8080/viewer/> |
| Arduino | `192.168.0.220`, UDP port 4210 | none | the calibration firmware | the Orin's `/arduino` page |

Each computer's folder holds the contents of its `rust/` folder here, so
`__live__/orin/rust/settings.toml` is `~/orin-cam/settings.toml` on the Orin. None
of this starts by itself after a reboot.

## Moving a file to the Pi or the Orin

Each part's README makes a package on the PC and expects it in the home folder
of the computer it's going to. There are two ways to move it.

**With `scp`,** which copies a file over SSH and asks for the login's password
unless the PC has a key set up. Replace the file and the address:

```bash
scp target/arduino-manager.tgz tony@192.168.0.247:~
```

**Through a terminal,** when `scp` isn't an option. A terminal can only carry
text, so the file is sent as **base64**, a way of writing any file as plain
letters and digits. Both packages reached the rig this way on 2026-10-31.

1. On the PC, in Git Bash, turn the file into text and print its **checksum**,
   a fingerprint that changes if a single byte does:

   ```bash
   base64 -w 76 target/arduino-manager.tgz > target/arduino-manager.b64
   ```

   ```bash
   sha256sum target/arduino-manager.tgz
   ```

2. In an SSH session on the other computer, start collecting the text. The
   prompt doesn't come back, because `cat` is now waiting for input:

   ```bash
   cat > ~/arduino-manager.b64
   ```

3. Paste the contents of the `.b64` file, press Enter if the last line hasn't
   ended, then press `Ctrl+D` on the empty line to finish. **Check that the
   prompt has come back.** Until it does, whatever you type goes into the file
   and not to the computer. Packages of up to 140 KB went through in pieces of
   60 lines, about a second apart. If you send the pieces from a Git Bash command
   rather than pasting them, first run `export MSYS_NO_PATHCONV=1`: otherwise Git
   Bash rewrites any piece that starts with `/` into a Windows path, which is the
   kind of damage the checksum in step 4 is there to catch.

4. Turn it back into the file, and check that its checksum matches step 1. If
   it doesn't, do not use the file; delete it and start again:

   ```bash
   base64 -d ~/arduino-manager.b64 > ~/arduino-manager.tgz
   ```

   ```bash
   sha256sum ~/arduino-manager.tgz
   ```
