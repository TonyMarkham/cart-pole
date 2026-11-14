# Deploying the Orin's webserver

The Orin has no monitor, so everything it shows, from the Arduino's pins to the
calibration of the rail, reaches a person through a web browser. This page puts
a new version of that webserver, the brain (the program that runs the balance
loop, `crates/brain/README.md`), its command-line tool `image-push` and its
browser pages onto the Orin, and checks that they work.

The work is split between two computers, because each builds what it can:

- **The PC** builds the browser pages. They are Rust compiled to WebAssembly
  (a form of program a browser can run), and the tools for that, `trunk` and the
  `wasm32-unknown-unknown` target, are only installed on the PC.
- **The Orin** builds the webserver, the brain and `image-push` itself, because
  they run on its own processor (aarch64 Linux) and the repo's rule is that
  robot-side code is built on the robot.

The Orin runs this folder from `~/orin-cam`, so
`__live__/orin/rust/settings.toml` here is `~/orin-cam/settings.toml` there.
After a deploy, `~/orin-cam` holds exactly the package from step 3, plus the
three things the Orin makes there itself:

- `target/`, the Orin's build output, which holds the running programs;
- `webserver.out` and `brain.out`, everything the server and the brain print
  (step 9);
- `pages/lens/stills/`, the photos the lens page's **Still** button saves.
  `calibrate_service.py` writes them there (`stills_dir` in
  `__live__/orin/python/settings.toml`), so the webserver can show them.

The Orin is `robo-orin`, at `192.168.0.215`, login `tony`.

> [!NOTE]
> ## Before the first deploy
>
> Only once on each computer, before its first deploy, not every time.
>
> > **A. On the PC: the WebAssembly target and `trunk`.** Rust itself comes from
> > [rustup](https://rustup.rs).
> >
> > > **A1. The WebAssembly target**, so Rust can compile for a browser:
> > >
> > > ```powershell
> > > rustup target add wasm32-unknown-unknown
> > > ```
> >
> > > **A2. `trunk`**, which builds each page and writes its HTML and WebAssembly
> > > files:
> > >
> > > ```powershell
> > > cargo install trunk --locked
> > > ```
>
> > **B. On the Orin: a C compiler and Rust.** The Orin doesn't need `trunk` or the
> > WebAssembly target, because the pages arrive already built.
> >
> > > **B1. The C compiler**, which some Rust dependencies need while they build:
> > >
> > > ```bash
> > > sudo apt install -y build-essential
> > > ```
> >
> > > **B2. Rust**, through rustup:
> > >
> > > ```bash
> > > curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
> > > ```
> > >
> > > > [!IMPORTANT]
> > > > The installer makes `cargo` findable only in sessions opened after it.
> > > > Close this SSH session and open a new one, or step 6 answers
> > > > `cargo: command not found`.
> >
> > > **B3. The GStreamer Data Protocol plugin**, which the brain's camera reader
> > > needs (`crates/brain/README.md`, *The camera*). It comes with
> > > `gstreamer1.0-plugins-bad`. Check:
> > >
> > > ```bash
> > > gst-inspect-1.0 gdppay
> > > ```
> > >
> > > It prints the plugin's details when it is there, and `No such element or
> > > plugin 'gdppay'` when it is not; then it needs installing, which is a
> > > decision for the person at the rig, not this page.
> >
> > > **B4. The Orin's Python**, `~/live-orin-python`, with its environment made
> > > and `src/policy.py`, `src/brain_worker.py` and `saved/policy.npz` in it: the
> > > brain starts its worker there (`settings.toml`, `[brain] worker_dir`). The
> > > folder is packed and copied from the PC by `__synthetic__/02-DEPLOY.md`'s
> > > Step 4 (or `__object_detection__/03-REAL_ORIN.md`'s Step 3); its
> > > environment is made once on the Orin by the command in
> > > `__live__/orin/python/readme-old.md`.

> ## Deploying
>
> The new version is unpacked and built in a folder of its own,
> `~/orin-cam-new`, so nothing in `~/orin-cam` changes before step 8: the running
> server, its pages and `image-push` stay exactly as they were, and a build that
> fails leaves them untouched. The pages are down only for the few seconds from
> step 7 to step 9. The version before is kept whole in `~/orin-cam-old`, for
> *Going back*.
>
> > [!IMPORTANT]
> > Run every step, in this order, each time.
>
> > ### On the PC
> >
> > > [!IMPORTANT]
> > > Every step on the PC runs in PowerShell, from `__live__/orin/rust`. The
> > > commands are written for Windows PowerShell 5.1, so they run in
> > > PowerShell 7 too. A command of several lines is pasted whole: PowerShell
> > > runs the lines one after another, and a block in `{ }` as one.
> >
> > > **1. Check that everything compiles.** Clippy is Rust's checker: it compiles
> > > the webserver, the brain and `image-push` and refuses the build over anything the
> > > workspace's `Cargo.toml` denies, such as an `unwrap()`. This folder is a
> > > Cargo **workspace** of its own (a group of crates built together, with one
> > > `Cargo.toml` for their dependencies), and Cargo picks the workspace from the
> > > folder it is started in: from the repo's root, the same command checks the
> > > root workspace and never looks at the webserver. The Orin builds only the
> > > files git tracks here (step 3), so this checks the same source the Orin
> > > builds only when git tracks every file the build needs; and it checks it on
> > > Windows, not Linux.
> > >
> > > > [!IMPORTANT]
> > > > Run this from `__live__/orin/rust`.
> > >
> > > ```powershell
> > > cargo clippy --workspace --all-targets
> > > ```
> >
> > > **2. Build the browser pages.** The pages are members of the repo's root
> > > workspace, not this one. Each page's `Trunk.toml` sends its output to
> > > `__live__/orin/rust/pages/<page>`, which is what the webserver serves. The
> > > six pages:
> > >
> > > - `viewer`, which shows each image `image-push` sends;
> > > - `arduino`, which draws the board and what each pin reads;
> > > - `calibrate`, which finds the rail's ends and centre, and measures the
> > >   markers at each;
> > > - `live`, the camera's picture with a box on every marker found, for
> > >   aiming the camera;
> > > - `lens`, which shows where to hold the printed board;
> > > - `run`, with Run, STOP and Re-centre for the balance loop, and the
> > >   brain's and the board's state.
> > >
> > > The command first deletes `pages/`, so it holds these six and nothing
> > > else: `pages/` is not in git, and a page that no longer exists would
> > > otherwise stay in it and be packed in step 3 (an earlier `pages/run`
> > > was, until 2026-11-06). Then it steps into each page's folder, `../wasm/<page>`, builds
> > > it, and steps back, even when a build fails.
> > >
> > > > [!IMPORTANT]
> > > > Start from `__live__/orin/rust`, in PowerShell: the command finds the
> > > > pages only from there.
> > >
> > > ```powershell
> > > if (Test-Path pages) { Remove-Item -Recurse -Force pages }
> > > foreach ($p in 'viewer','arduino','calibrate','live','lens','run') {
> > >     Push-Location ../wasm/$p
> > >     trunk build --release
> > >     Pop-Location
> > > }
> > > ```
> > >
> > > > [!WARNING]
> > > > A failed build doesn't stop the next one. Check each page's output for
> > > > an error before going on to step 3.
> >
> > > **3. Pack the package** into `target/orin-webserver.tgz` at the repo's root.
> > > It holds the files git tracks in this folder (`git ls-files`), the pages
> > > from step 2, and a file `VERSION` naming the commit they come from.
> > > Nothing else goes in: not a file git doesn't track, not the build output,
> > > not the IDE's settings.
> > >
> > > `VERSION` is the commit's short id, such as `71a2da2b`. It ends in `-dirty`
> > > when a file git tracks anywhere in the repo has changes not yet committed:
> > > the package then holds those changes, and no commit matches it exactly. The
> > > command prints `VERSION` at the end, for step 10.
> > >
> > > The command writes `VERSION` and a list of what to pack,
> > > `orin-webserver.list`, into `target/` at the repo's root, then hands the
> > > list to `tar`. The list ends with `-C ../../../target` and `VERSION`: `tar`
> > > reads `-C` on a line of its own as "the names after this are in that
> > > folder", which is how `VERSION` goes in without being written into this
> > > folder.
> > >
> > > > [!IMPORTANT]
> > > > Run this from `__live__/orin/rust`. Anywhere else, it packs the wrong
> > > > folder.
> > >
> > > ```powershell
> > > New-Item -ItemType Directory -Force ../../../target | Out-Null
> > > git describe --always --dirty | Set-Content -Encoding ascii ../../../target/VERSION
> > > git ls-files | Set-Content -Encoding ascii ../../../target/orin-webserver.list
> > > Add-Content -Encoding ascii ../../../target/orin-webserver.list 'pages', '-C', '../../../target', 'VERSION'
> > > tar -czf ../../../target/orin-webserver.tgz -T ../../../target/orin-webserver.list
> > > Get-Content ../../../target/VERSION
> > > ```
> > >
> > > > [!WARNING]
> > > > PowerShell runs each line even when the one before failed. If anything
> > > > is printed before `VERSION`, it is an error: fix it and run the step
> > > > again.
> > >
> > > > [!NOTE]
> > > > A new file reaches the Orin only once git tracks it. Until then the
> > > > Orin's build doesn't see it, even though step 1 on the PC does.
> >
> > > **4. Move the package** to the Orin's home folder, `~`, with `scp`, which
> > > copies a file over SSH.
> > >
> > > > [!IMPORTANT]
> > > > Run this from `__live__/orin/rust`: the package's path,
> > > > `../../../target/orin-webserver.tgz`, starts there.
> > >
> > > ```powershell
> > > scp ../../../target/orin-webserver.tgz tony@192.168.0.215:~
> > > ```
> > >
> > > > [!NOTE]
> > > > It asks for the login's password unless the PC has an SSH key set up.
>
> > ### On the Orin
> >
> > > [!IMPORTANT]
> > > Every step on the Orin runs in an SSH session on the Orin
> > > (`ssh tony@192.168.0.215`), and can be started from any folder: each
> > > command names its folders in full, from `~`, and the ones that must run
> > > inside a folder go there themselves with `cd`, inside round brackets
> > > `( … )`: the brackets run the command in a copy of the shell, so its `cd`
> > > moves only the copy, and your shell is still in the folder it started in.
> > > A command of several lines is pasted whole: a line ending in `&&` carries
> > > on to the next, which runs only if that line worked.
> >
> > > **5. Unpack the package** into a new, empty `~/orin-cam-new`. Any
> > > `~/orin-cam-new` already there is removed first: it is what an earlier
> > > deploy left when its build failed, or what *Going back* set aside.
> > >
> > > > [!IMPORTANT]
> > > > Run this on the Orin, from any folder: every path in it starts at `~`.
> > >
> > > ```bash
> > > rm -rf ~/orin-cam-new &&
> > > mkdir ~/orin-cam-new &&
> > > tar -xzf ~/orin-webserver.tgz -C ~/orin-cam-new
> > > ```
> >
> > > **6. Build the server, the brain and the command-line tool** in
> > > `~/orin-cam-new`, from nothing. Cargo keeps what it compiles in `target/`,
> > > and the new folder has none, so it compiles every library the three
> > > programs use as well as the programs themselves: nothing from an earlier
> > > build can reach this one. The programs land in
> > > `~/orin-cam-new/target/release/`, where steps 9 and 11 run them from once
> > > the folder has become `~/orin-cam`.
> > >
> > > ```bash
> > > (cd ~/orin-cam-new && cargo build --release -p webserver -p brain -p image-push)
> > > ```
> > >
> > > > [!CAUTION]
> > > > If the build fails, stop here. Nothing in `~/orin-cam` has changed, and
> > > > the running server is still the old one. Fix the cause on the PC, and
> > > > start again from step 1.
> >
> > > **7. Stop the running server and the brain.**
> > >
> > > > [!WARNING]
> > > > From here until step 9, every page is down, and every viewer loses its
> > > > last pushed image. Never do this during a run: STOP the run first.
> > >
> > > `pkill -x webserver` ends the program whose name is exactly `webserver`,
> > > and `pkill -x brain` the brain. Both run in the background (step 9), with
> > > no terminal of their own, so this is the way to reach them. The brain's
> > > worker, a Python child of the brain, ends when the brain does.
> > >
> > > ```bash
> > > pkill -x webserver; pkill -x brain
> > > ```
> >
> > > **8. Swap the folders.** The version before the last one, in
> > > `~/orin-cam-old`, is removed. The current version becomes `~/orin-cam-old`,
> > > the new one becomes `~/orin-cam`, and the lens photos move across to it,
> > > because they are the Orin's own and the package has none. `mkdir -p`
> > > makes sure `pages/lens/` exists to take them: left in `~/orin-cam-old`,
> > > they would be removed by the next deploy.
> > >
> > > ```bash
> > > (
> > >   cd ~ &&
> > >   rm -rf orin-cam-old &&
> > >   if [ -d orin-cam ]; then mv orin-cam orin-cam-old; fi &&
> > >   mv orin-cam-new orin-cam &&
> > >   if [ -d orin-cam-old/pages/lens/stills ]; then
> > >     mkdir -p orin-cam/pages/lens &&
> > >     mv orin-cam-old/pages/lens/stills orin-cam/pages/lens/
> > >   fi
> > > )
> > > ```
> > >
> > > > [!NOTE]
> > > > Anything else that was only in the old `~/orin-cam` stays in
> > > > `~/orin-cam-old`, and the next deploy's step 8 removes it. Something to
> > > > keep on the Orin belongs outside these three folders.
> >
> > > **9. Start the server and the brain in the background,** so they keep
> > > running after this SSH session ends. Each looks for `settings.toml` in the
> > > folder it is started in, and the server for the pages in `pages/` there:
> > > without `settings.toml` it stops at once with an error, and without
> > > `pages/` every page answers 404. That is why the commands go to
> > > `~/orin-cam` first, inside the brackets.
> > >
> > > The rest of the command cuts the server loose from this session: `setsid`
> > > gives it a session of its own and `nohup` keeps it running when this one
> > > closes; `< /dev/null` gives it no keyboard; `> webserver.out 2>&1` sends
> > > everything it prints into `~/orin-cam/webserver.out`; and the `&` lets the
> > > prompt come back at once.
> > >
> > > ```bash
> > > (
> > >   cd ~/orin-cam &&
> > >   nohup setsid ./target/release/webserver > webserver.out 2>&1 < /dev/null &
> > > )
> > > ```
> > >
> > > The brain the same way. It starts its Python worker first, which loads the
> > > TensorRT engine and the policy, and only then listens; that takes a few
> > > seconds.
> > >
> > > ```bash
> > > (
> > >   cd ~/orin-cam &&
> > >   nohup setsid ./target/release/brain > brain.out 2>&1 < /dev/null &
> > > )
> > > ```
> > >
> > > > [!NOTE]
> > > > Neither starts by itself after the Orin reboots: run this step again.
> > >
> > > Their first messages, read from the files:
> > >
> > > ```bash
> > > cat ~/orin-cam/webserver.out
> > > ```
> > >
> > > > [!SUCCESS]
> > > > `Server started at http://0.0.0.0:8080`.
> > >
> > > ```bash
> > > cat ~/orin-cam/brain.out
> > > ```
> > >
> > > > [!SUCCESS]
> > > > A `worker ready:` line naming the policy, then
> > > > `brain idle at http://127.0.0.1:8091`.
> > >
> > > > [!FAILURE]
> > > > A line `Error: Worker { message: "…" … }`: the worker could not start,
> > > > and `message` says why; if it says the worker ended before its ready
> > > > line, the worker's own message is above it in the file. The usual
> > > > causes are the Python folder or its environment missing (B4), or the
> > > > engine or `saved/policy.npz` not where the Python's `settings.toml`
> > > > says.

> ## Checking the deploy
>
> > **10. Confirm the new server is the one running.** Every later check tests
> > whichever server is running, so this one comes first. Both answers are on
> > the Orin. The first names the program the running server was started from;
> > the second is the package's `VERSION`.
> >
> > ```bash
> > readlink /proc/$(pgrep -x webserver)/exe
> > ```
> >
> > ```bash
> > cat ~/orin-cam/VERSION
> > ```
> >
> > > [!SUCCESS]
> > > `/home/tony/orin-cam/target/release/webserver`, and the same `VERSION` that
> > > step 3 printed on the PC.
> >
> > > [!FAILURE]
> > > `/home/tony/orin-cam-old/target/release/webserver`: the old server is still
> > > running. Redo steps 7 and 9.
> >
> > > [!FAILURE]
> > > Nothing at all: no server is running. Its last messages are in
> > > `~/orin-cam/webserver.out`; fix the cause, then redo step 9.
>
> > **11. Push one image to the viewer.** An image arriving in the viewer proves
> > the viewer page, its WebSocket and `image-push` all work.
> >
> > > [!IMPORTANT]
> > > A page answering `200` is not proof. The browser runs a page's script only
> > > if it matches the checksum in that page's `index.html`, so a broken page
> > > still answers `200`.
> >
> > > **11a. In the browser, open the viewer** at
> > > <http://192.168.0.215:8080/viewer/>. It shows "waiting for image" until the
> > > first push.
> > >
> > > > [!NOTE]
> > > > Already open? Refresh it: the viewer doesn't reconnect after a server
> > > > restart.
> >
> > > **11b. On the Orin, push one** of the test images the package carries in
> > > `assets/`:
> > >
> > > ```bash
> > > ~/orin-cam/target/release/image-push ~/orin-cam/assets/blue.png
> > > ```
> > >
> > > > [!SUCCESS]
> > > > The viewer turns blue.
>
> > **12. Confirm the Orin reaches the Arduino.** On the Orin, ask the board
> > its state: one `state` datagram to the address and port in
> > `settings.toml`'s `[arduino]`, and its answer printed. The board's
> > firmware, `controller`, answers `state` in every state. It does not answer
> > `pins` yet (`__documentation__/real-loop-dia.md`, *What exists today*), so the webserver's
> > `/arduino/pins` says `no answer within 1000 ms` even when the board is
> > there.
> >
> > ```bash
> > python3 -c "import socket; s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM); s.settimeout(2); s.sendto(b'state', ('192.168.0.220', 4210)); print(s.recvfrom(512)[0].decode())"
> > ```
> >
> > > [!SUCCESS]
> > > The board's state, such as `{"status":200,"result":"calibrated","message":"the rail is known and the cart is at rest","pin":null,"pulses":0}`.
> > > After a reset it is `"uncalibrated"`, and in FAULT the FAULT STATE reply:
> > > each is an answer, so the board is reached.
> >
> > > [!FAILURE]
> > > `TimeoutError` (`socket.timeout` on older Python): no answer in 2 s. The
> > > board is off, not on the WiFi, or at another address.
>
> > **13. Open the other pages** in the browser. Refresh any that were already
> > open.
> >
> > | Page | Shows when it works |
> > |---|---|
> > | <http://192.168.0.215:8080/> | A link to each page |
> > | <http://192.168.0.215:8080/arduino> | The board drawn; its pin readings need `pins`, which `controller` does not answer yet |
> > | <http://192.168.0.215:8080/calibrate> | The rail, its switches and the moves |
> > | <http://192.168.0.215:8080/live/> | The camera's picture, with a box on every marker found |
> > | <http://192.168.0.215:8080/lens/> | The camera's picture, with the red box to hold the board in |
> > | <http://192.168.0.215:8080/run/> | STOP, Run, Re-centre, and the brain's and the board's state; Run and Re-centre are greyed out, with the reason beside them, unless the brain is idle and the board calibrated. Do not press Run or Re-centre here: they move the cart |
> >
> > > [!NOTE]
> > > The calibration's measurements and the `live` and `lens` pages need
> > > `calibrate_service.py`, which runs the camera and the detector and is
> > > started on its own, not by this page. Without it, the server answers them
> > > with "the calibration service did not answer, so it may not be running".
>
> > **14. Ask the brain its state.** From the PC, in PowerShell. The webserver
> > forwards this to the brain on the Orin's own address, and, while the brain
> > is idle, adds the board's answer to `state` as `board`.
> >
> > ```powershell
> > curl.exe -s http://192.168.0.215:8080/run/status
> > ```
> >
> > > [!SUCCESS]
> > > `{"state":"idle","message":"","run_dir":null,"frames":0,"board":{"status":200,"result":"calibrated",...}}`.
> > > Without an answer from the board, `board` is null and `board_error`
> > > says why; step 12 checks the board itself.
> >
> > > [!FAILURE]
> > > "the brain did not answer, so it may not be running": step 9's brain
> > > command, and `~/orin-cam/brain.out`.

> [!WARNING]
> ## Going back
>
> For a new version that misbehaves. This puts the whole version before back:
> its program, the brain, `image-push`, pages, settings and source, as step 8 kept them in
> `~/orin-cam-old`. The lens photos move back with it. Only one version back is
> kept.
>
> > **R1. Stop the server and the brain**, as in step 7.
>
> > **R2. Swap the folders back.** The new version is set aside as
> > `~/orin-cam-new`, which the next deploy's step 5 removes. The command first
> > checks that `~/orin-cam-old` exists (`[ -d orin-cam-old ]`), and does
> > nothing if it doesn't: after the first deploy, or after going back once,
> > there is no version to go back to, and swapping anyway would leave no
> > `~/orin-cam` at all.
> >
> > ```bash
> > (
> >   cd ~ &&
> >   [ -d orin-cam-old ] &&
> >   mv orin-cam orin-cam-new &&
> >   mv orin-cam-old orin-cam &&
> >   if [ -d orin-cam-new/pages/lens/stills ]; then
> >     mkdir -p orin-cam/pages/lens &&
> >     mv orin-cam-new/pages/lens/stills orin-cam/pages/lens/
> >   fi
> > )
> > ```
>
> > **R3. Start the server and the brain**, as in step 9.
