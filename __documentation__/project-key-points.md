# Project Key Points

> ## 1. Mechanical Design
>
> - Used FreeCAD to mechanically model the entire rig in a CAD Tool
> - Use the [Fasteners Workbench](https://wiki.freecad.org/Fasteners_Workbench) Addon in FreeCAD to add all of the Nuts, Bolts and Washers
> - Added assembly nodes to make it easier to dynamically add ArUco Markers in Unity
> - Added assembly nodes to make it easier to identify abd pace MuJoCo Sites
> - Exported to STEP (noticed FreeCAD does not export density properly to STEP)
> - Built a custom Python tool that reads the density in FreeCad and injects it properly to the exported STEP file.

> ## 2. OpenCascade C Application Binary Interface (ABI)
>
> - For downstream tooling, I wanted to use Rust, but OpenCascade is a C++ Codebase
> - I created the git repository: [occt-dev](https://github.com/TonyMarkham/occt-dev) to document how to build OpenCascade on Windows, Linux and mac so each would have a known-good linkable library to work from
> - The git repo contains a [v0.1.0](https://github.com/TonyMarkham/occt-dev/releases/tag/v0.1.0) release with all of the libraries that will be needed pre-compiled
> - In [greyhound-snake](https://github.com/TonyMarkham/greyhound-snake) I build a small C Application Binary Interface (ABI) to expose the features of OpenCascade (OCCT) that I would need

> ## 3. OpenCV C Application Binary Interface (ABI)
>
> - For downstream tooling, I wanted to use Rust, but OpenCV is a C++ Codebase
> - I created the git repository: [opencv-dev](https://github.com/TonyMarkham/opencv-dev) to document how to build OpenCV on Windows, Linux and mac so each would have a known-good linkable library to work from
> - The git repo contains a [v0.1.0](https://github.com/TonyMarkham/opencv-dev/releases/tag/v0.1.0) release with all of the libraries that will be needed pre-compiled
> - In [greyhound-snake](https://github.com/TonyMarkham/greyhound-snake) I build a small C Application Binary Interface (ABI) to expose the features of OpenCV that I would need

> ## 4. Rust STEP to Mesh Utility
> - In [greyhound-snake](https://github.com/TonyMarkham/greyhound-snake) I built a Rust Utility that transforms STEP data into Tessellated meshes using OCCT

> ## 5. Unity STEP ScriptedImporter
> - In [greyhound-snake](https://github.com/TonyMarkham/greyhound-snake) I built Unity Scripted Importer for STEP. Now, any STEP file in Unity Assets folder would automatically be turned into a Unity GameObject in the AssetDatabase, leaving the underlying STEP file untouched.

> ## 6. MuJoCo MJCF File Authoring (Unity UI)
> - In [greyhound-snake](https://github.com/TonyMarkham/greyhound-snake) I build ScriptableObjects with UIToolkit Custom Editors to streamline the creation of MuJoCo elements like:
>     - joints
>     - actuators
>     - sites
> - Created an affordance in the top-level MuJoCo ScriptableObject to export a MoJoCo-Compliant MJCF file

> ## 7. Augmented Reality University of Cordoba (ArUco) Markers - Object Detection
> - 4x 50mmx50mm ArUco markers were chosen to mark the positions of:
>     1. The Pole's pivot
>     2. The Pole's Tip
>     3. The Rail's Idler End
>     4. The Rail's Motor End
> - At the 2m distance that the camera needed to be to see the entire rig, the standard OpenCV's ArUco detection was not capable of identifying the 4x 50mm markers
> - Created 4x 3D Models of Desks:
>     - Classic
>     - Panel-End
>     - Standard
>     - Writing
> - Downloaded 3x HDRI's from Polyhaven:
>     - [Neon Photo Studio](https://polyhaven.com/a/neon_photostudio)
>     - [PAV Studio 03](https://polyhaven.com/a/pav_studio_03)
>     - [Story Studio 04](https://polyhaven.com/a/story_studio_04)
> - Created a series of custom ScriptableObjects that would: 
>     - Position ArUco Makers
>     - Randomize Camera Position, Roll, Pitch & Yaw
>     - Randomize Directional Light Pitch & Yaw
>     - Randomize which the Desk
>     - Randomize the HDRI and it's Y-Rotation
>     - Randomize Rig's Cart Position and Pole Rotation
> - Created a Image Randomizer to generate `n` blocks of `y` images with all of the above parameters randomized on fixed, rotating seeds
> - Created a total of 11,000 random images: 10,000 training set and a 1,000 validation set
> - Added an 8K image gen to the loop and passed through an OpenCV ArUco detector as a rubric to identify if one of the markers was obscured by the rotated pole.
> - Exact Labeling of classes from the Unity Transform (`camera.WorldToViewportPoint(...)`), using the 8K step to exclude obscured markers from the labels
> - Trained a YOLO Model ~20 hours on the 4 classes
> - Tested on the real rig with a 100% hit rate.

> ## 8. Statistically Accurate Timing Measurements
> - Created scripts and test jigs to isolate the anticipated event timings throughout the Observation > Action loop:
>     - Camera - Sensor Exposure to Image
>     - Hand-off - Image from Sensor to Python Object Detection Worker
>     - Object Detection - Image In > Boxes out
>     - Build Data - Build the Policy's 36 inputs
>     - Policy Inference - Generate next motor command
>     - Wifi Data Hop - Orin > Arduino
>     - Motor Change - motor runs at new rate
> - Isolated to 5x Randomizable Timings:
>     - Stretch 1: Image Capture
>     - Stretch 2: Image Received > Command Sent
>     - Stretch 3: Command Sent > Motor Change
>     - Stretch 4: Wait for Camera Sensor to be ready
>     - Wildcard: 1:3,000 chance that the something fails and no new command gets applied

> ## 9. Reward Calculation
> - Use the DeepMind [dm_control / cartpole.py](https://github.com/google-deepmind/dm_control/blob/main/dm_control/suite/cartpole.py)
> - This project's reward: `__synthetic__/trainer/python/dm_control_reward.md`

> ## 10. Synthetic Training
> - Built a training loop where every part stands in for a piece of the real robot:
>     - Orchestrator (Rust) - the loop's driver; stands in for the physical world and times every event
>     - MuJoCo - the physics, run in-process from the MJCF exported by Unity
>     - Unity Player (`SyntheticCamera.exe`, as `-player-mode SyntheticCamera`) - the camera; renders each frame of the rig at 728x544 greyscale
>     - Synthetic Orin (Python + TensorRT) - finds the markers with the same YOLO model the robot runs
>     - Synthetic Arduino (Rust) - the stepper's pulse rate, its acceleration limit and the rail's limit switches
>     - Trainer (Python, Stable-Baselines3 SAC) - the learner
> - The policy only sees what the robot would see: 36 inputs, the detected marker positions plus its own last 6 commands, never MuJoCo's true state
> - Every step's timing is drawn at random from the rig's measured stretches (section 8), including the 1:3,000 lost command
> - Ran 12 training instances side by side (a 3x4 grid of Unity windows), all feeding one policy
> - Each episode starts with the pole hanging and lasts 500 steps (~21 s): the policy has to swing it up and then hold it
> - Final run (`20261108_115227`): 6,000,000 steps, 12,039 episodes, ~19.5 hours

> ## 11. Custom Embedded Rust HAL for the Arduino UNO R4
> - To write the Arduino's firmware in Rust, I built [unor4-hal](https://github.com/TonyMarkham/unor4-hal): a Hardware Abstraction Layer (the library that drives the board's pins, timers and serial lines) for the UNO R4's Renesas RA4M1 processor, written from scratch
> - Drivers: GPIO, polled and interrupt-driven UART, PWM, ADC, USB serial, data flash, a stepper-motor step generator (STEP, DIR, ENA) and limit-switch inputs
> - **Compile-Time Hardware Validation**, using the typestate builder pattern and `const` evaluation: a wrong pin, timer or value fails at `cargo check` with a message naming it, before anything reaches the board
>     - Typestate: what each pin can do is part of its type, so asking for something it can't do won't compile (`` `A0` has no GPT output ``), and neither will handing one pin to two drivers (`` `D5` is already used by another driver ``)
>     - `const` evaluation: the builder's numbers (pulse timing, pull-ups, board loads) are checked by the compiler itself, when the setup is written as a named `const`
> - Proven on real hardware with a two-board validation rig: an Arduino UNO R3 checks every driver on the R4 (33/33 checks)
> - Published on crates.io

> ## 12. Forked the Arduino WiFi Bridge to Add Interrupts
> - The UNO R4 WiFi's WiFi runs on a separate chip that talks to the main processor over a serial line, through Arduino's bridge firmware
> - The stock bridge only answers when asked, so the processor would have to keep polling for new datagrams
> - Forked it (branch `feature/interrupt`) to add push mode: each UDP datagram is sent to the processor as it arrives, and the HAL's interrupt-driven UART (`UartIrq`) wakes it
> - Push mode covers UDP only, so the Orin's commands are datagrams, not web requests

> ## 13. Arduino Controller in Embedded Rust
> - Wrote the UNO R4 WiFi firmware (`controller`) in Rust, on the HAL:
>     - Takes each command from the Orin as one 3-byte UDP datagram over WiFi
>     - Sleeps until an interrupt wakes it, never polls
>     - Moves the motor's rate towards each command by at most the jump limit (900,000 pulses/s²), the same limit the policy trained with
>     - Finds the rail's ends with two limit switches, keeps the cart off them with a soft limit, and stops with a FAULT if one trips

> ## 14. Arduino Manager on a Raspberry Pi
> - The rig's Arduino sits in another room, and its full record of what happened goes to a USB port nobody is watching
> - Built `arduino-manager` in Rust, a service on a Raspberry Pi plugged into the Arduino by USB, so the board can be logged and reflashed from the desk:
>     - Logs every line on the Arduino's USB serial line to a timestamped file, a new file each time the line opens
>     - A web page showing the serial line and the log
>     - `POST /flash` takes the firmware the PC built and uploads it with `arduino-cli`, so the PC needs no Arduino tools of its own

> ## 15. Building and Deploying the Real Rig
> - Replaced the Raspberry Pi 5 with a Jetson Orin Nano Super: TensorRT FP16 finds the markers in 8–10 ms per frame, against 80–125 ms for the best Pi 5 options
> - Replaced the DRV8825 stepper driver with a DM542: the DRV8825 simply could not move the NEMA 23 motor
> - Built the brain (`__live__/orin/rust/crates/brain`): a Rust program on the Orin that runs the same loop the synthetic one did, calling Python only for TensorRT detection and the policy
>     - Camera frame (728x544 greyscale) > JPEG round-trip, as in training > YOLO > Pivot & End to metres > 6-entry pool > 36 inputs > policy > command to the Arduino
> - Calibrated the camera: a lens calibration to undo the lens's bend, then a homography (the matrix turning pixels into metres in the pole's plane); worst point 0.20 px
> - Built web pages on the Orin in Rust compiled to WebAssembly, with no JavaScript: live view, lens, calibrate, Arduino, and `/run` with Run, STOP and Re-centre
> - Exported the trained policy (`policy.npz`) from the training run into the repo, for the Orin to load
> - First run on the rig (2026-11-09): the 6M-step policy swung the pole up in 3.4 s and held it within 10° of upright for the rest of a 30 s run