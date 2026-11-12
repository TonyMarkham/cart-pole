# generate — making a new batch of training frames

The detector ([02-TRAIN.md](02-TRAIN.md)) learns to find the rig's four
markers from thousands of camera frames, each with a box drawn around every
marker. Nobody draws those boxes: Unity renders the rig at random poses, on
random desks, under random light, and writes each frame with its boxes already
known, because Unity placed the markers itself. This file makes one such
**batch**: a set of frame folders plus the `data.yaml` training reads.

It reads the generation sets in `__synthetic__/generator/` and writes the batch
to `datasets/generated/`, beside this file.

## How a batch is made

Two programs work together:

- **The Unity player**, `SyntheticCamera.exe`: the rig's Unity scene built
  into a program that runs without the Unity Editor. Given a **generation set**
  (how many frames, which desks and lights, a random seed), it renders that
  many frames and writes them, with their labels, into one `run_<set>_<time>/`
  folder.
- **The generator**, a Rust program: it starts several players at once, each in
  its own window, hands each one a set, hands it the next when it finishes, and
  when no set is left writes `data.yaml`, the list of runs training reads.

```text
__synthetic__/generator/
├── master.json             a complete generation set: every setting, written out
├── sets/                   the pool: one Set-NN.json per set, holding only what
│                           differs from master.json (its name, its seed)
├── sets-smoke/             two 10-frame sets, for a first quick run (Step 3)
└── rust/
    ├── generator.toml      the generator's settings: the pool, the val set, the
    │                       output folder, the player, how many players at once
    └── generator-smoke.toml the same for the smoke run
unity/projects/SyntheticCamera/
└── Build/SyntheticCamera.exe   the Unity player (not in git, made by Step 1)
__object_detection__/datasets/generated/
├── run_Set-NN_<time>/      one per set: images/, labels/, markers.json
└── data.yaml               the batch: which runs are train, which val
```

A **seed** is the number a set's random choices start from. The same set with
the same seed makes the same frames again (`datasets/generated/.gitignore`:
the corpus is "regenerable by design"), so a batch with new frames needs new
seeds or new sets (Step 2).

> ## The PC's terminal
>
> Every command below runs in one PowerShell window on the PC. Open it and go to
> the repo root:
>
>> ### PowerShell
>> ```pwsh
>> cd D:/git/cart-pole
>> ```
>
> Step 1 also uses the Unity Editor, which has no terminal.

> ## Step 1 — build the Unity player (only when the Unity side changed) (not run yet)
>
> The player is a snapshot of the Unity project and its packages at the moment it
> was built. Rebuild it when anything it is made from has changed since:
> `unity/projects/SyntheticCamera/`, the packages in `unity/packages/`, or the
> greyhound packages it loads. Both kinds of package are the `file:` entries in
> `unity/projects/SyntheticCamera/Packages/manifest.json`: its
> `file:../../../packages/…` paths lead from that `Packages/` folder up to
> `unity/packages/`. A change to the generation sets does not need a
> rebuild: the generator sends each set to the player while it runs.
>
> First see when the player was last built:
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> Get-Item unity/projects/SyntheticCamera/Build/SyntheticCamera.exe | Select-Object LastWriteTime
>> ```
>
> To rebuild it:
>
> > [!IMPORTANT]
> > Run in the Unity Editor, with the project `unity/projects/SyntheticCamera/` open
>
>> ### Unity Editor
>>
>> 1. Open the project `unity/projects/SyntheticCamera/` in Unity
>>    6000.3.28f1, the version its `ProjectSettings/ProjectVersion.txt` names.
>> 2. **File > Build Profiles**, with **Windows** as the platform. Its scene
>>    list holds the one scene, `Assets/Scenes/SyntheticCamera.unity`.
>> 3. **Build**, and choose the folder `unity/projects/SyntheticCamera/Build/`.
>>    Unity names the program after the project's Product Name,
>>    `SyntheticCamera`, so it writes `SyntheticCamera.exe` there: the path
>>    `player` names in `generator.toml`.
>
>> ### Expected Results:
>> The `LastWriteTime` command above now shows the time of this build.

> ## Step 2 — choose the sets (not run yet)
>
> Each file in `__synthetic__/generator/sets/` is one set, and each set becomes
> one run folder. A set file holds only what differs from `master.json`; the
> generator lays it over the master to get the full set. In the 2026-11-12
> batch every set differs only by its name and seed:
>
> ```json
> {
>   "name": "Set-02",
>   "seed": 2002
> }
> ```
>
> `Set-01.json` has no seed, so it takes the master's, 1001. Every set takes
> the master's `"states": 1000`, the number of frames it makes.
>
> - **New frames from the same sets**: give each set a new `seed`.
> - **More frames**: add a set, `Set-22.json`, with its own name and seed.
> - **A different look** (desks, skyboxes, light, camera): change the key in
>   `master.json` for every set, or put the key in one set's file for that set
>   alone. Every key a set can hold is defined once, in
>   `__synthetic__/generator/proto/generation_set.proto`.
>
> **The val set**: training scores the model on frames it never learns from,
> the **val** frames. `val` in `__synthetic__/generator/rust/generator.toml`
> names which sets those are, `["Set-21.json"]` today; every other set is
> **train**. If you add or rename sets, check that `val` still names one of
> them.

> ## Step 3 — a smoke run, to check everything works (not run yet)
>
> A **smoke run** is a tiny run that tries every part once, so a broken player
> or a bad set shows up in seconds instead of after an hour. It uses
> `generator-smoke.toml`: the two 10-frame sets in `sets-smoke/`, written to
> `target/generator-smoke/`, which git ignores, never to the training corpus.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> cargo run --release -p generator --bin generator -- --config __synthetic__/generator/rust/generator-smoke.toml
>> ```
>
>> ### What each part means
>>
>> - **`cargo run --release`**: builds the Rust program, optimised, if it is not
>>   built already, then runs it.
>> - **`-p generator --bin generator`**: which program: the `generator` crate
>>   has two, and `generator` is the one that makes batches.
>> - **`--`**: everything after it goes to the generator, not to cargo.
>> - **`--config …/generator-smoke.toml`**: the generator's settings. The paths
>>   inside it are read from that file's own folder, so it runs the same from
>>   any folder.
>
>> ### Expected Results:
>> Two player windows open side by side at the top-left of the screen, render,
>> and close.
>> The terminal prints one line per event; each starts with the time and
>> `INFO`, and the words to look for are, in order:
>>
>> ```text
>> pool loaded sets=2 …
>> starting players players=2 sets=2 window=960x704
>> connected …            (once per player)
>> job sent … set=Smoke-01 states=10
>> run done … frames=10 …  (once per set)
>> stopped …              (once per player: its window closes)
>> dataset written runs=2 data_yaml=__synthetic__/generator/rust\../../../target/generator-smoke\data.yaml
>> ```
>>
>> The last line's path is `generator-smoke.toml`'s folder followed by its
>> `output_dir`, `../../../target/generator-smoke`: three folders up from
>> `__synthetic__/generator/rust` is the repo root, so it is
>> `target/generator-smoke/data.yaml`.
>>
>> If it stops early, its last line says why. Each player's own Unity log is
>> `target/generator-smoke/logs/player_<k>.log`, `<k>` counting from 0.

> ## Step 4 — generate the batch (not run yet)
>
> The full pool, written to `__object_detection__/datasets/generated/`.
>
> > [!WARNING]
> > This replaces `datasets/generated/data.yaml`. The previous batch's run
> > folders stay where they are, but the new `data.yaml` lists only the runs
> > this step makes. To keep the previous batch whole, move its `run_*` folders
> > and its `data.yaml` out of `datasets/generated/` first, the way
> > `datasets/generated/archive/` holds the 2026-11-10 batch.
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> cargo run --release -p generator --bin generator -- --config __synthetic__/generator/rust/generator.toml
>> ```
>
> The command is Step 3's, with the real settings file.
>
>> ### Expected Results:
>> Six player windows open in two rows of three, filling the screen
>> (`instance_rows = 2`, `instance_columns = 3` in `generator.toml`). Each one
>> takes the next set when it finishes one, until all 21 are done. The same
>> lines as Step 3, now:
>>
>> ```text
>> pool loaded sets=21 …
>> starting players players=6 sets=21 window=1280x1056
>> run done … frames=1000 …  (once per set)
>> stopped …              (once per player: its window closes)
>> dataset written runs=21 data_yaml=__synthetic__/generator/rust\../../../__object_detection__/datasets/generated\data.yaml
>> ```
>>
>> How long it takes: in the 2026-11-12 batch, by its run folders' names, the
>> last of the 21 sets started 22 minutes after the first.
>>
>> If it stops early, its last line says why, and it writes no `data.yaml`.
>> Each player's own Unity log is `target/generator-logs/player_<k>.log`.

> ## Step 5 — check the batch (not run yet)
>
> `data.yaml` is what training reads: the train runs, the val runs, the four
> class names, and `channels: 1` (every frame is greyscale).
>
> > [!IMPORTANT]
> > Run in the PC's terminal, from the repo root, `D:/git/cart-pole/`
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> Get-Content __object_detection__/datasets/generated/data.yaml
>> ```
>
>> ### Expected Results:
>> ```text
>> train:
>> - run_Set-01_<time>
>> …
>> - run_Set-20_<time>
>> val:
>> - run_Set-21_<time>
>> names:
>> - idler
>> - motor
>> - pivot
>> - tip
>> channels: 1
>> ```
>
> Then count each listed run's frames and labels:
>
>> ### PowerShell (the PC's terminal)
>> ```pwsh
>> $dataset = "__object_detection__/datasets/generated"
>> Select-String -Path "$dataset/data.yaml" -Pattern '^- (run_.+)$' |
>>     ForEach-Object { $run = $_.Matches[0].Groups[1].Value; "{0}  images {1}  labels {2}" -f $run, (Get-ChildItem "$dataset/$run/images").Count, (Get-ChildItem "$dataset/$run/labels").Count }
>> ```
>
>> ### What each part means
>>
>> - **`Select-String … '^- (run_.+)$'`**: finds each line of `data.yaml` that
>>   names a run, `- run_…`, and keeps the run's name. The class names are
>>   list lines too, but do not start with `run_`, so they are skipped.
>> - **`ForEach-Object { … }`**: for each run, counts the files in its
>>   `images/` and `labels/` folders.
>
>> ### Expected Results:
>> One line per run, each with as many images as labels, and as many as the
>> set's `states`, 1000:
>>
>> ```text
>> run_Set-01_<time>  images 1000  labels 1000
>> …
>> run_Set-21_<time>  images 1000  labels 1000
>> ```
>>
>> A label file can be empty: a frame where no marker's box was confirmed, a
>> background frame, is still a training frame.
>
> The batch is ready to train on: [02-TRAIN.md](02-TRAIN.md), Step 3.
