# The Orin's Rust

The Orin has no monitor, so this is how it shows things on a web browser. A command-line tool sends a PNG to a server on the Orin, and the server pushes it over a WebSocket to every open viewer page. The same server has a page showing the Arduino's pins and a page that calibrates the rail. The browser side is written in Rust and compiled to WebAssembly. This began as the orin-cam proof of concept, which is why the Orin keeps it in `~/orin-cam`.

```text
image-push  --HTTP POST /image-->  webserver  --WebSocket /ws-->  viewer (Rust WASM in the browser)
```

## Crates

This folder is the workspace pushed to the Orin and built there, so it can't depend on anything outside itself. The browser pages are built on the PC, from `../wasm`, and only their output reaches this folder, in `pages/`.

| Crate | Built on | What it does |
|---|---|---|
| `crates/image-push` | Orin | Command-line tool that sends a PNG to the server |
| `crates/webserver` | Orin | Receives images, pushes them to viewers, serves the pages, asks the Arduino for its pins and asks it to move the cart |
| `../wasm/arduino` (`arduino-page`) | PC | Runs in the browser, draws the Arduino and its pins, and updates them from each reading |
| `../wasm/calibrate` (`calibrate-page`) | PC | Runs in the browser, draws the rail, and has the buttons that find its ends and the centre |
| `../wasm/lens` (`lens-page`) | PC | Runs in the browser: measures the lens with the printed board, held where a red box shows, flat and then turned |
| `../wasm/live` (`live-page`) | PC | Runs in the browser: the camera's live picture with a box on every marker the detector finds, for aiming the camera |
| `../wasm/viewer` | PC | Runs in the browser, opens the WebSocket and shows each image |

## Server routes

The server listens on port `8080` (`[server] address` in `settings.toml`). On the Orin, that's `http://192.168.0.215:8080`.

| Route | Method | Purpose |
|---|---|---|
| `/` | GET | Hello page: a link to each page below |
| `/viewer/` | GET | Viewer page |
| `/ws` | GET | WebSocket that pushes images to viewers |
| `/image` | POST | Accepts a new PNG, up to 16 MB (`[server] max_image_bytes`) |
| `/arduino` | GET | The Arduino's pins: a drawing of the board with each pin the rig uses and what it reads now |
| `/arduino/pins` | GET | One reading of those pins, asked of the Arduino over UDP at that moment |
| `/arduino/settings` | GET | `[arduino]` from `settings.toml`, for the pages |
| `/calibrate` | GET | The calibration page: the rail, its switches and the moves |
| `/calibrate/motor` | POST | **Moves the cart** to the motor's end, and answers when it has stopped |
| `/calibrate/idler` | POST | **Moves the cart** to the idler's end, and answers when it has stopped |
| `/calibrate/centre` | POST | **Moves the cart** to the rail's centre, and answers when it has stopped. `?speed=5000` asks for that many pulses per second |
| `/calibrate/stop` | POST | Asks the board to end the move under way. 200 means only that the messages left; the move's own request gets the `stopped` answer |
| `/calibrate/state` | GET | What the moves have found: the span and the stop the cart is at, so a refreshed page knows them too |
| `/calibrate/measure` | POST | Starts measuring a pose: `?stop=motor&pole=horizontal&count=30&gap_mm=5`. The centre takes no gap and has only `pole=hanging`. Refused while the cart is moving. Stops the live view, which has the camera until then |
| `/calibrate/measure/status` | GET | How the measurement is going, with the latest frame and its boxes |
| `/calibrate/measure/accept` | POST | Keeps the finished measurement for its pose |
| `/calibrate/measure/cancel` | POST | Ends any measurement, and throws its result away |
| `/calibrate/fit` | POST | Fits the matrix from the seven kept poses and the span, and stores it if it passes |
| `/live` | GET | The live view: the camera's picture with every marker the detector finds, for aiming the camera |
| `/live/start` | POST | Starts the live view. Refused while a measurement is running |
| `/live/stop` | POST | Stops the live view |
| `/lens` | GET | The lens measurement: a red box over the live picture, with the board's shape dashed inside it, shows where and how to hold the printed board |
| `/lens/start`, `/lens/stop` | POST | Starts or stops the lens view. Starting stops the live view; refused while a measurement is running |
| `/lens/status` | GET | The latest frame, the box to fill and the board's shape there, the markers and corners found, and the stills kept. Asking keeps the view on |
| `/lens/skip` | POST | Moves the box on without keeping a still |
| `/lens/restart` | POST | Forgets every kept still |
| `/lens/solve` | POST | OpenCV's `calibrateCamera` on the kept stills; stores `lens.json` |
| `/live/status` | GET | The latest frame, its boxes, and how often each marker was found lately. Asking keeps the view on: it stops by itself when no page has asked for `[live] idle_stop_s` |

## The Arduino page

The Orin has no monitor and the rig's Arduino should run without a USB cable, so wiring faults need another way to be seen. `/arduino` (on the Orin, <http://192.168.0.215:8080/arduino>) draws the board, marks the pins the rig uses and shows what each reads, updating about once a second.

```text
page in the browser  --GET /arduino/pins-->  webserver  --UDP "pins"-->  Arduino
                                                        <--UDP levels--
```

The server sits in the middle because a browser cannot send UDP, and UDP is what wakes the board (the firmware's README, *What it answers*). If no answer arrives within `timeout_ms`, the page says so and shows no levels, then asks again.

The page is Rust compiled to WebAssembly, like the viewer, with no hand-written JavaScript. Its explanations and styling are in `../wasm/arduino/index.html`, its pin facts in `src/pins/mod.rs`, and its drawing in `src/render.rs`. `trunk` builds it into `pages/arduino`, which the server reads from disk. It needs the Arduino running the calibration firmware, with the interrupt bridge on its WiFi chip.

`settings.toml`, in this folder, says where the Arduino is and how often to ask. The server reads it once at start, so restart the server after a change.

A reading happens only while the page is open; the board sleeps otherwise.

## The calibration page

The stepper can't report where the cart is, so the board has to find both ends of the rail before it can find the middle. `/calibrate` (on the Orin, <http://192.168.0.215:8080/calibrate>) is where a person asks for those moves and watches what comes back. It draws the rail and its two switches, has one button for each move and a full-width STOP, and lists every answer the board gave. **Each move button moves the real cart.** STOP is a message over WiFi, which can be lost, so stand at the e-stop as well.

```text
page in the browser  --POST /calibrate/motor-->  webserver  --UDP "CalibrateMotorTravelLimit"-->  Arduino
                                                                       the cart moves
                     <--the answer, same JSON--            <--UDP answer, once it has stopped--
```

The page's own text explains the order to press them in and how to read each answer. The firmware's README has every answer the board can give.

**The HTTP status is the board's.** The webserver returns the board's JSON unchanged, under the `status` it holds: 200, 400, 408 or 409. Two more come from the webserver, with the same five keys:

| Status | `result` | Meaning |
|---|---|---|
| 409 | `busy` | A move is already under way, so this one was not sent |
| 504 | `no_answer` | No answer within the wait. The answer may have been lost on the WiFi, so the cart may have moved |
| 502 | `no_answer` | The message could not be sent |

**How long it waits.** `move_timeout_ms` in `settings.toml`, which must be longer than the firmware's `seek_timeout_ms`. A centring with a speed waits longer by half the rail (`half_span_pulses`) at that speed.

**During a move** the board throws away every message except a stop. So the webserver holds one move at a time, even if the page that asked is closed, and `/arduino/pins` answers `503` without sending anything. A page refreshed during a move sees that, and leaves only STOP on until the move has ended.

**One exchange at a time.** The board answers whoever sent the latest datagram, so a message that lands while it is answering another takes that answer. So a move waits for any reading on its way to finish before it is sent, and only a move's own answer, one with a `status`, ends the wait for it. On 2026-11-01, before this, a move sent during a reading got the pin levels as its answer, and the page put its buttons back while the cart was still moving.

**STOP.** `/calibrate/stop` sends the board the `Stop` datagram `stop_sends` times, `stop_gap_ms` apart, because any one can be lost. The first to arrive ends the move, and the rest are ignored. It sends even if the server knows of no move, because a server restarted mid-move has forgotten the one under way.

**What it doesn't know.** The board has no message that reports its span, so the span and the "what to do next" line on the page come from the answers that page has seen. A refreshed page starts knowing nothing, though the board remembers. The board decides every move all the same, and answers `not_ready` or `already_there` when it won't make one.

The page is Rust compiled to WebAssembly, with no hand-written JavaScript. Its explanations and styling are in `../wasm/calibrate/index.html`, what it knows in `src/session/mod.rs`, and its drawing in `src/render.rs`.

### Measuring a stop

The camera's pixels only become metres through a matrix, and fitting it takes Pivot and End measured at three places the cart is known to be. So when a move ends as asked, a window opens over the page, and nothing behind it can be pressed:

1. At each end, it asks for the gap from the cart to the rail support beside it, measured by hand, in millimetres. It asks you to steady the pole, and you press **NEXT**. The centre has no gap to measure.
2. It asks how many images to use, from 30 to 9,999. Only an image with both Pivot and End in it counts. You press **START**.
3. It shows a progress bar and the latest frame, with Pivot's box in green and End's in blue.
4. It shows a summary. **Done** keeps the result and closes the window, ready for the next move. **Retry** throws it away and starts this stop again.

**Cancel** is always there, and leaves the cart where it is. **Measure this stop** on the page opens the window again without moving anything. After the third stop's Done, the fit runs and shows how close each point came.

```text
page  --POST /calibrate/measure-->  webserver  --POST /measure-->  calibrate_service.py  -->  camera, GPU
```

This needs [the calibration service](../python/README.md) running. **The cart can't move during a measurement:** each side holds the one permit while it checks the other, so a move is refused with `busy`, and a measurement is refused while the cart is moving. `[calibration]` in `settings.toml` says where the service is. With no service running, the moves still work, and the window says so at NEXT.

## Prerequisites

### On the development PC

Rust, installed with [rustup](https://rustup.rs), plus the WebAssembly target and `trunk`, which builds the browser pages:

```bash
rustup target add wasm32-unknown-unknown
```

```bash
cargo install trunk --locked
```

### On the Orin

Rust, installed with rustup, and a C compiler. The Orin doesn't need `trunk` or the WebAssembly target, because the browser pages are built on the PC.

```bash
sudo apt install -y build-essential
```

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Deployment

The Orin keeps this folder's contents in `~/orin-cam`. Run every step, in this order, each time you deploy. The server keeps running until step 8, so the page and viewer are only down for the few seconds between steps 8 and 9, and a failed build leaves them untouched.

### On the PC, from this folder

**1. Check that everything compiles.**

```bash
cargo clippy --workspace --all-targets
```

**2. Build the browser pages.** These are members of the repo's root workspace, not this one. Each writes its page and WebAssembly files to its own folder in `pages/`, and `pages/` is not in git, so all five are needed the first time.

```bash
cd ../wasm/viewer && trunk build --release && cd ../../rust
```

```bash
cd ../wasm/arduino && trunk build --release && cd ../../rust
```

```bash
cd ../wasm/calibrate && trunk build --release && cd ../../rust
```

```bash
cd ../wasm/live && trunk build --release && cd ../../rust
```

```bash
cd ../wasm/lens && trunk build --release && cd ../../rust
```

**3. Pack the project.** In Git Bash, this writes `target/orin-calibration.tgz` at the repo's root. Build output stays behind, because the Orin builds its own.

```bash
tar --exclude=target --exclude=./.idea -czf ../../../target/orin-calibration.tgz .
```

**4. Move the package** to the Orin's home folder ([how](../../README.md)).

### On the Orin

**5. Back up what's there,** both the source and the running program. The date and time go in the names, so an earlier backup is never overwritten.

```bash
cd ~ && tar --exclude=target -czf orin-cam-backup-$(date +%Y%m%d-%H%M%S).tgz orin-cam && cp orin-cam/target/release/webserver orin-cam-webserver-backup-$(date +%Y%m%d-%H%M%S)
```

**6. Unpack the package.** Files of the same name are replaced and nothing is deleted, so anything only on the Orin, such as `crates/wip`, stays.

```bash
tar -xzf ~/orin-calibration.tgz -C ~/orin-cam
```

**7. Build the server and the command-line tool.** The running server isn't disturbed. If this fails, stop here and fix it.

```bash
cd ~/orin-cam && cargo build --release -p webserver -p image-push
```

**8. Stop the running server.** This clears the last pushed image from every viewer.

```bash
pkill -x webserver
```

If it was started in a terminal you can see (the first command in step 9), `Ctrl+C` there does the same. If it was started in the background (the second command), `Ctrl+C` signals don't reach it, so `pkill` is the way.

**9. Start the server.** Run it from `~/orin-cam`, because the paths to `pages/` and `settings.toml` are relative; without `settings.toml` it stops at once with an error. Started this way, it stops if this SSH session ends:

```bash
cd ~/orin-cam && ./target/release/webserver
```

To keep it running after the SSH session ends, which is how it runs on the Orin now, start it this way instead. Its messages go to `webserver.out`, and it still doesn't start by itself after a reboot:

```bash
cd ~/orin-cam && (nohup setsid ./target/release/webserver > webserver.out 2>&1 < /dev/null &)
```

You should see `Server started at http://0.0.0.0:8080`, on screen or in `webserver.out`.

### In the browser

**10. Open or refresh the viewer** at <http://192.168.0.215:8080/viewer/>, the Arduino page at <http://192.168.0.215:8080/arduino>, and the calibration page at <http://192.168.0.215:8080/calibrate>.

Refresh is required if a page was already open, because the viewer doesn't reconnect after a server restart. The viewer shows "waiting for image" until the first push. **Push one image to prove it works** (*Switch images*): the page can answer `200` and still be broken, because the browser only runs the script if it matches the checksum in `index.html`.

### Check the deployment

**11. Confirm the server is live.** From the PC:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://192.168.0.215:8080/image
```

`405` means the server is running and has the `/image` route, which only accepts POST. `404` means the old server is still running.

**12. Confirm it reaches the Arduino.** From the PC, this should print the pins' levels, such as `{"d2":0,"d3":0,"d4":1,"d5":1,"d6":1}`. Otherwise it prints why it got none:

```bash
curl -s http://192.168.0.215:8080/arduino/pins
```

### Going back

If the new version misbehaves, stop the server (step 8), put the saved program back, and start it (step 9). Replace `<date-time>` with the one in the file's name:

```bash
cp ~/orin-cam-webserver-backup-<date-time> ~/orin-cam/target/release/webserver
```

## Copy the assets to the Orin

This puts the contents of `assets/` into `~/Downloads` on the Orin, for the commands below. It overwrites files with the same name and deletes nothing. The package in deployment step 3 carries them, so on the Orin, after step 6:

```bash
mkdir -p ~/Downloads && cp ~/orin-cam/assets/* ~/Downloads/
```

## Switch images

Each command replaces the image in every open viewer, and that image stays until the next push. `assets.xcf` is a source file, not a PNG, so `image-push` refuses it.

On the Orin, if the server is using its own terminal, run these in a second SSH session. They need steps 7 and 9 of the deployment, and the copy above.

```bash
~/orin-cam/target/release/image-push ~/Downloads/blue.png
```

```bash
~/orin-cam/target/release/image-push ~/Downloads/cyan.png
```

```bash
~/orin-cam/target/release/image-push ~/Downloads/green.png
```

```bash
~/orin-cam/target/release/image-push ~/Downloads/magenta.png
```

```bash
~/orin-cam/target/release/image-push ~/Downloads/red.png
```

```bash
~/orin-cam/target/release/image-push ~/Downloads/yellow.png
```

`image-push` sends to `http://192.168.0.215:8080` by default. To send somewhere else, add `--server`:

```bash
~/orin-cam/target/release/image-push --server http://localhost:8080 ~/Downloads/blue.png
```

## Troubleshooting

| What you see | What it means | What to do |
|---|---|---|
| `Address already in use` when starting the server | A server is already running, and the old one keeps serving | Stop it (deployment step 8), then start it again |
| `http status: 404` from `image-push` | The old server is still running | Redo deployment steps 8 and 9 |
| `http status: 413` from `image-push` | The PNG is over 16 MB, `[server] max_image_bytes` | Use a smaller file |
| `Connection refused` from `image-push` | The server isn't running | Redo deployment step 9 |
| `[Not A PNG]` from `image-push` | The file doesn't start with a PNG signature | Use a PNG file |
| `/viewer/` returns 404 | `pages/viewer` wasn't built or pushed, or the server was started outside `~/orin-cam` | Redo deployment steps 2 to 6, or step 9 from `~/orin-cam` |
| A page stays blank, and the browser's console says `Failed to find a valid digest in the 'integrity' attribute` | Something has changed the script in that page's folder in `pages/`, or the browser has kept a bad copy | Press `Ctrl+F5`. If it persists, redo deployment steps 2 to 6 |
| `/arduino/` returns 404 | `pages/arduino` wasn't built or pushed, or the server was started outside `~/orin-cam` | Redo deployment steps 2 to 6, or step 9 from `~/orin-cam` |
| `/calibrate` returns 404 | `pages/calibrate` wasn't built or pushed, or the server was started outside `~/orin-cam` | Redo deployment steps 2 to 6, or step 9 from `~/orin-cam` |
| The calibration page shows `no_answer` after a move | The answer was lost or the board is off. The cart may have moved | Look at the switches on the page and at the cart. Find both ends again if in doubt |
| The window says the calibration service did not answer | `calibrate_service.py` isn't running | Start it ([its README](../python/README.md)) |
| The server stops at start with `[IO Error]` or `[Settings Error]` | `settings.toml` is missing, or something in it is wrong | Start it from `~/orin-cam`, and check that file |
| The Arduino page says "No answer from the board" | The board is off, on other firmware or at another address, or a message was lost | Check the firmware's README, *How it runs* |
| The page stays on "waiting for image" after a push | The page has a dead connection | Refresh the page |
| `[WebSocket Error]: ... Connection reset without closing handshake` in the server log | A client disconnected abruptly | Nothing, the server keeps running |

## Known limitations

- **No security:** anyone on the network can view the page or replace the image. There's no login, and traffic isn't encrypted.
- **Images aren't saved:** the latest image is only held in memory, so viewers are blank after a server restart until the next push.
- **No reconnect:** a viewer has to be refreshed after a server restart.
- **Server doesn't start on boot:** after the Orin restarts, it has to be started again (deployment step 9).
- **PNG check is basic:** both `image-push` and the server only check the first 8 bytes of the file.
- **Duplicated values:** `WS_ROUTE`, `IMAGE_ROUTE` and `PNG_SIGNATURE` are defined in more than one crate and must be kept in sync by hand. `VIEWER_ROUTE` must also match `public_url` in `../wasm/viewer/Trunk.toml`. `PINS_REQUEST` must match the calibration firmware, and the three move datagrams and `STOP_REQUEST` the controller firmware; the Arduino's UDP port must match the firmware it runs. The `CALIBRATE_*_ROUTE`s must match the page's `constants.rs`. `service` in `[calibration]` must match `[service]` in `../python/settings.toml`.
