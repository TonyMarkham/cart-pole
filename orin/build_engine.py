"""Build the detector's TensorRT engine from the float ONNX (orin-plan.md R2).

An engine only runs on the GPU and TensorRT version it was built with, so
each machine builds its own from the same best.onnx; the Windows TensorRT
wheels ship no trtexec, hence this script. FP16 is the robot's precision
(detector-plan.md Results: FP16 matched the reference within 0.27 px).
FP16 here is the builder flag (TensorRT may still keep a layer in FP32),
the same as trtexec --fp16 on the Orin.

After writing the engine the script loads it back and checks its tensors
against the config: input [1, 1, input_size, input_size], output
[1, 4 + classes, N].

usage: python build_engine.py <ObjectDetection.toml> <engine-out>
"""

import sys
from pathlib import Path

import tensorrt as trt

from detector_config import DetectorConfig


def build(config: DetectorConfig, engine_path: Path, logger: trt.Logger) -> None:
    builder = trt.Builder(logger)
    network = builder.create_network(0)
    parser = trt.OnnxParser(network, logger)
    if not parser.parse_from_file(str(config.model)):
        errors = "\n".join(str(parser.get_error(i)) for i in range(parser.num_errors))
        raise RuntimeError(f"{config.model}: ONNX parse failed:\n{errors}")
    builder_config = builder.create_builder_config()
    builder_config.set_flag(trt.BuilderFlag.FP16)
    serialized = builder.build_serialized_network(network, builder_config)
    if serialized is None:
        raise RuntimeError("engine build failed (see the TensorRT log above)")
    engine_path.write_bytes(bytes(serialized))


def check(config: DetectorConfig, engine_path: Path, logger: trt.Logger) -> None:
    engine = trt.Runtime(logger).deserialize_cuda_engine(engine_path.read_bytes())
    if engine is None:
        raise RuntimeError(f"{engine_path}: does not load")
    shapes = {}
    for i in range(engine.num_io_tensors):
        name = engine.get_tensor_name(i)
        mode = "input" if engine.get_tensor_mode(name) == trt.TensorIOMode.INPUT else "output"
        shapes[mode] = (name, list(engine.get_tensor_shape(name)))
        print(f"{mode:6s} {name}: {shapes[mode][1]} {engine.get_tensor_dtype(name)}")
    size = config.input_size
    if shapes["input"][1] != [1, 1, size, size]:
        raise RuntimeError(f"input shape {shapes['input'][1]}, expected [1, 1, {size}, {size}]")
    out = shapes["output"][1]
    if len(out) != 3 or out[:2] != [1, 4 + len(config.classes)]:
        raise RuntimeError(f"output shape {out}, expected [1, {4 + len(config.classes)}, N]")


def main() -> None:
    config_path, engine_path = map(Path, sys.argv[1:])
    config = DetectorConfig.load(config_path)
    logger = trt.Logger(trt.Logger.WARNING)
    print(f"TensorRT {trt.__version__}: {config.model} -> {engine_path} (FP16)")
    build(config, engine_path, logger)
    check(config, engine_path, logger)
    print(f"ok: {engine_path} ({engine_path.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
