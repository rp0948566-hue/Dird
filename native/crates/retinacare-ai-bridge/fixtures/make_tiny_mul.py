"""Development-time export of a synthetic, NON-MEDICAL ONNX fixture.

Writes `tiny_mul.onnx`: result = image * 2.0 for a float32 [1, 3, 4, 4]
tensor. The graph has no training, no learned weights and no clinical
meaning of any kind; it exists only so the Rust loader, manifest gate and
session path can be exercised end to end. It is NOT a retinal model and
proves nothing about HierarchiRetina, screening, grading or accuracy.

Run (development-only virtualenv, never part of the production runtime):

    ai_engine/.venv/bin/python native/crates/retinacare-ai-bridge/fixtures/make_tiny_mul.py
"""

from pathlib import Path

import onnx
from onnx import TensorProto, helper, checker

SHAPE = [1, 3, 4, 4]
OUTPUT = Path(__file__).with_name("tiny_mul.onnx")


def main() -> None:
    graph = helper.make_graph(
        nodes=[
            helper.make_node(
                "Mul",
                inputs=["image", "scale"],
                outputs=["result"],
                name="multiply_by_two",
            )
        ],
        name="tiny_mul_nonmedical",
        inputs=[helper.make_tensor_value_info("image", TensorProto.FLOAT, SHAPE)],
        outputs=[helper.make_tensor_value_info("result", TensorProto.FLOAT, SHAPE)],
        initializer=[helper.make_tensor("scale", TensorProto.FLOAT, [], [2.0])],
    )
    model = helper.make_model(
        graph,
        producer_name="retinacare-p01-fixture",
        opset_imports=[helper.make_opsetid("", 13)],
    )
    model.ir_version = 8  # ONNX 1.13 semantics
    checker.check_model(model)
    onnx.save(model, str(OUTPUT))
    print(f"wrote {OUTPUT} ({OUTPUT.stat().st_size} bytes)")


if __name__ == "__main__":
    main()