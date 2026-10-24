"""The robot's marker detector: JPEG in, the 4-slot marker frame out.

Python + TensorRT is how the robot detects (AGENTS.md, The real robot,
exception), and the synthetic Orin runs this same module on the PC's GPU.
It must compute what Ultralytics' predictor computes on best.onnx
(synthetic-orin-plan.md O1, detector-plan.md D1-D3):

- decode greyscale with OpenCV;
- letterbox to input_size: r = min(S / h, S / w), resized (round(w r),
  round(h r)) with INTER_LINEAR, padding split round(d/2 - 0.1) before and
  round(d/2 + 0.1) after, filled with pad_value; then / 255;
- slot rule D3: per class the candidate with the highest score, detected
  when that score >= confidence, its center mapped back through the
  letterbox, x = (cx - left) / r, y = (cy - top) / r, and normalized by the
  frame's width and height; otherwise (-1, -1). No NMS.

For a 728x544 frame: r = 0.879121, resized 640x478, 81 rows top and bottom.

A frame that does not decode, or is not the size the slots are normalized
to, raises FrameError (O3): "not detected" only ever means the model found
nothing.
"""

from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np
import tensorrt as trt
from cuda.bindings import runtime as cudart

from detector_config import DetectorConfig

NOT_DETECTED = (-1.0, -1.0)


class FrameError(ValueError):
    """The frame is not a JPEG of the expected size: an error, never empty slots."""


@dataclass(frozen=True)
class MarkerFrame:
    slots: list[tuple[float, float]]  # class order; normalized center or (-1, -1)
    confidences: list[float]  # each class's best score, also when below the threshold


def _check(result):
    """cuda-python returns (error, value...); raise on any error."""
    err, *rest = result if isinstance(result, tuple) else (result,)
    if err != cudart.cudaError_t.cudaSuccess:
        raise RuntimeError(f"CUDA error {err}")
    return rest[0] if len(rest) == 1 else rest


class Detector:
    def __init__(self, config: DetectorConfig, engine_path: Path, frame_size: tuple[int, int]):
        self.config = config
        self.frame_size = frame_size
        size = config.input_size
        w, h = frame_size
        self._r = min(size / h, size / w)
        self._resized = (round(w * self._r), round(h * self._r))
        self._left = round((size - self._resized[0]) / 2 - 0.1)
        self._top = round((size - self._resized[1]) / 2 - 0.1)
        self._canvas = np.full((size, size), config.pad_value, np.uint8)

        logger = trt.Logger(trt.Logger.WARNING)
        self._engine = trt.Runtime(logger).deserialize_cuda_engine(engine_path.read_bytes())
        if self._engine is None:
            raise RuntimeError(f"{engine_path}: does not load (built for another GPU or TensorRT?)")
        self._context = self._engine.create_execution_context()
        names = [self._engine.get_tensor_name(i) for i in range(self._engine.num_io_tensors)]
        inp = next(n for n in names if self._engine.get_tensor_mode(n) == trt.TensorIOMode.INPUT)
        outp = next(n for n in names if self._engine.get_tensor_mode(n) == trt.TensorIOMode.OUTPUT)
        in_shape = list(self._engine.get_tensor_shape(inp))
        out_shape = list(self._engine.get_tensor_shape(outp))
        if in_shape != [1, 1, size, size] or len(out_shape) != 3 or out_shape[:2] != [1, 4 + len(config.classes)]:
            raise RuntimeError(f"{engine_path}: tensors {in_shape} -> {out_shape} do not fit the config")
        self._input = np.empty((1, 1, size, size), np.float32)
        self._output = np.empty(out_shape[1:], np.float32)  # rows: cx, cy, w, h, class scores
        self._d_in = _check(cudart.cudaMalloc(self._input.nbytes))
        self._d_out = _check(cudart.cudaMalloc(self._output.nbytes))
        self._context.set_tensor_address(inp, self._d_in)
        self._context.set_tensor_address(outp, self._d_out)
        self._stream = _check(cudart.cudaStreamCreate())

    def detect(self, jpeg: bytes) -> MarkerFrame:
        grey = cv2.imdecode(np.frombuffer(jpeg, np.uint8), cv2.IMREAD_GRAYSCALE)
        if grey is None:
            raise FrameError("frame does not decode as an image")
        h, w = grey.shape
        if (w, h) != self.frame_size:
            raise FrameError(f"frame is {w}x{h}, expected {self.frame_size[0]}x{self.frame_size[1]}")
        nw, nh = self._resized
        self._canvas[self._top : self._top + nh, self._left : self._left + nw] = cv2.resize(
            grey, (nw, nh), interpolation=cv2.INTER_LINEAR
        )
        np.divide(self._canvas, 255.0, out=self._input[0, 0], dtype=np.float32)
        self._infer()

        scores = self._output[4:]
        best = scores.argmax(axis=1)
        slots, confidences = [], []
        for c, i in enumerate(best):
            conf = float(scores[c, i])
            confidences.append(conf)
            if conf >= self.config.confidence:
                x = (float(self._output[0, i]) - self._left) / self._r / w
                y = (float(self._output[1, i]) - self._top) / self._r / h
                slots.append((x, y))
            else:
                slots.append(NOT_DETECTED)
        return MarkerFrame(slots, confidences)

    def _infer(self) -> None:
        h2d = cudart.cudaMemcpyKind.cudaMemcpyHostToDevice
        d2h = cudart.cudaMemcpyKind.cudaMemcpyDeviceToHost
        _check(cudart.cudaMemcpyAsync(self._d_in, self._input.ctypes.data, self._input.nbytes, h2d, self._stream))
        if not self._context.execute_async_v3(self._stream):
            raise RuntimeError("TensorRT execute_async_v3 failed")
        _check(cudart.cudaMemcpyAsync(self._output.ctypes.data, self._d_out, self._output.nbytes, d2h, self._stream))
        _check(cudart.cudaStreamSynchronize(self._stream))

    def close(self) -> None:
        _check(cudart.cudaFree(self._d_in))
        _check(cudart.cudaFree(self._d_out))
        _check(cudart.cudaStreamDestroy(self._stream))
