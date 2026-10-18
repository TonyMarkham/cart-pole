# Appendix — Camera calibration for a new setup

**Appendix — not on the path. Run whenever the rig's camera is
mounted, re-mounted, or the setup around it changes.**

## Purpose

A new camera setup changes the pixels the pipeline consumes, and the
pipeline's acceptance test is not a calibration chart — it is **the
trained models themselves**. The rig's markers are the reference
object: point the camera at the rig, capture frames, auto-label with
the deployment model, and look. If the 640 finds all four markers and
classes them correctly in the new setup, the setup is calibrated —
because "the deployment model detects the markers" is precisely the
property the pipeline needs. No coordinate math, no chessboard: the
models are the calibration target.

The calibration dataset is a **pre-named, pre-populated skeleton** —
`datasets/calibration/` ships with its `data.yaml` (legend + pointer)
and placeholder-tracked `images\` and `labels\` folders. Nothing is
dynamic: every session copies its frames in, evaluates, and the next
session replaces them. Calibration frames are scratch — never scored,
never trained on.

> ## Step 1 — capture frames in the new setup
>
> Why: the calibration must run on the pixels the Pi will consume in
> the new setup — same raw-path rules as the main loop.
>
> - Capture with the Pi utility (`D:\git\webrtc-pi`) under the raw
>   camera path: 1456×1088 native, 1 ms shutter, AE/AWB locked —
>   the rules are 06-REAL_LABELING.md Step 1.
> - Pose the rig so **all four markers are visible and unoccluded** —
>   pole mid-swing, away from both rail markers. A calibration frame
>   with an occluded marker cannot distinguish "camera problem" from
>   "marker hidden."
> - The utility downloads the session as a tar and expands it into
>   its run folder with the variant subfolders, as always.

> ## Step 2 — land the half frames (manual)
>
> Why: the deployment model runs at the Pi's operating point — the
> half-sized frames through the 640 are exactly what the Pi will
> consume. Calibration that passes here is calibration for
> deployment.
>
> Copy — do not move — the `half-grey` variant folder's frames into:
>
> ```
> datasets\calibration\data\images\
> ```
>
> The destination is always literal. The source is the utility's
> expanded run folder — the one path that changes per session.

> ## Step 3 — auto-label with the deployment model
>
> Why: the 640 is the model the Pi runs — if it detects the markers
> in the new setup, the setup works for deployment. The only
> difference from the main loop's predict commands is the fixed
> `name=calibration`:
>
> ```powershell
> uv run yolo detect predict model=D:/git/cart-pole/training/runs/cp2-640-base/weights/best.onnx source=D:/git/cart-pole/datasets/calibration/data/images save_txt save_conf save=True imgsz=640 project=D:/git/cart-pole/training/runs name=calibration
> ```
>
> Expected: `training\runs\calibration\labels\` — one 6-field
> proposal file per frame. Reusing the name auto-increments
> (`calibration2`, ...) — convert before the next predict run.

> ## Step 4 — convert
>
> Why: the viewer reads corpus-format labels. The converter handles
> all three datasets — calibration included whenever its proposals
> exist:
>
> ```powershell
> uv run python src\convert_proposals.py
> ```
>
> Expected: a `calibration` block in the output — one line per frame,
> then `calibration: N label files written, M empty, to ...`.

> ## Step 5 — evaluate in the YOLO Labeler
>
> Why: the human decides. Open `datasets\calibration\data` in the
> **YOLO Labeler** (`D:\git\greyhound-snake\crates\yolo-labeler`) and
> walk the frames.
>
> What **calibrated** looks like: every frame shows all four markers
> (Left, Right, Pivot, End), correct classes, tight boxes — the same
> clean standard as the 06 evaluation.
>
> What **failure** looks like, and its fixes:
>
> - **Markers clipped at frame edges** — the rig sits outside the
>   field of view: re-aim or re-mount the camera.
> - **Markers consistently missed or misclassified** — the camera is
>   too far, too angled, or the exposure drifted: re-position, then
>   re-check exposure against 06 Step 1's rules.
> - **Blur or ghosting** — the capture path degraded into streaming:
>   the batch is disqualified; re-capture through the raw download.
> - **Isolated mistakes** — noise: ignore.
>
> A clean walk means the setup is calibrated — the deployment model
> — the model the Pi runs — accepts the new setup. Log nothing, keep
> nothing: the calibration tree is scratch, replaced by the next
> session.
