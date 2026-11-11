"""Checks the worker's framing with a pretend detector and policy: the bytes the
brain reads are exactly what the contract says (__documentation__/real-loop-dia.md, Contracts, the
brain's worker). Needs no camera, GPU or TensorRT.

usage (from __live__/orin/python): uv run python src/test_brain_worker.py
"""

import io
import json
import struct
import unittest

import numpy as np

from brain_worker import (
    FRAME_REQUEST,
    POLICY_REQUEST,
    error_line,
    pack_action,
    pack_detections,
    pack_not_found,
    serve,
    unpack_inputs,
)

FRAME_BYTES = 728 * 544
INPUTS = 36


class Framing(unittest.TestCase):
    def test_the_detections_are_twenty_little_endian_f32_in_class_order(self) -> None:
        slots = [(0.1, 0.2), (-1.0, -1.0), (0.5, 0.6), (0.7, 0.8)]
        sizes = [(0.01, 0.02), (0.0, 0.0), (0.03, 0.04), (0.05, 0.06)]
        confidences = [0.9, 0.1, 0.8, 0.7]
        data = pack_detections(slots, sizes, confidences)
        self.assertEqual(len(data), 80)
        values = struct.unpack("<20f", data)
        self.assertAlmostEqual(values[0], 0.1, places=6)
        self.assertAlmostEqual(values[1], 0.2, places=6)
        self.assertEqual(values[5], -1.0)
        self.assertEqual(values[6], -1.0)
        self.assertAlmostEqual(values[9], 0.1, places=6)
        self.assertAlmostEqual(values[19], 0.7, places=6)

    def test_a_failed_frame_is_every_class_not_found(self) -> None:
        values = struct.unpack("<20f", pack_not_found())
        for c in range(4):
            self.assertEqual(values[c * 5 : c * 5 + 4], (-1.0, -1.0, 0.0, 0.0))

    def test_the_inputs_are_little_endian_f64(self) -> None:
        inputs = np.arange(INPUTS, dtype=np.float64) / 10
        data = inputs.astype("<f8").tobytes()
        np.testing.assert_array_equal(unpack_inputs(data, INPUTS), inputs)

    def test_the_action_is_one_little_endian_f64(self) -> None:
        self.assertEqual(struct.unpack("<d", pack_action(-0.25)), (-0.25,))

    def test_the_error_line_is_json_with_the_reason(self) -> None:
        self.assertEqual(json.loads(error_line("no policy file")), {"error": "no policy file"})


class Serving(unittest.TestCase):
    def test_a_frame_then_the_inputs_are_answered_in_turn_until_the_input_closes(self) -> None:
        seen = {}

        def detect(pixels: bytes) -> bytes:
            seen["pixels"] = pixels
            return pack_not_found([0.1, 0.2, 0.3, 0.4])

        def act(inputs: np.ndarray) -> float:
            seen["inputs"] = inputs
            return 0.5

        frame = bytes([7]) * FRAME_BYTES
        inputs = np.linspace(-1, 1, INPUTS)
        inp = io.BytesIO(FRAME_REQUEST + frame + POLICY_REQUEST + inputs.astype("<f8").tobytes())
        out = io.BytesIO()

        serve(inp, out, detect, act, FRAME_BYTES, INPUTS)

        self.assertEqual(seen["pixels"], frame)
        np.testing.assert_array_equal(seen["inputs"], inputs)
        answer = out.getvalue()
        self.assertEqual(len(answer), 80 + 8)
        self.assertAlmostEqual(struct.unpack("<20f", answer[:80])[4], 0.1, places=6)
        self.assertEqual(struct.unpack("<d", answer[80:]), (0.5,))

    def test_an_input_that_closes_mid_frame_ends_quietly(self) -> None:
        inp = io.BytesIO(FRAME_REQUEST + bytes(100))
        out = io.BytesIO()
        serve(inp, out, lambda pixels: pack_not_found(), lambda inputs: 0.0, FRAME_BYTES, INPUTS)
        self.assertEqual(out.getvalue(), b"")

    def test_an_unknown_request_is_an_error(self) -> None:
        with self.assertRaises(ValueError):
            serve(io.BytesIO(b"X"), io.BytesIO(), lambda pixels: b"", lambda inputs: 0.0, FRAME_BYTES, INPUTS)


if __name__ == "__main__":
    unittest.main()
