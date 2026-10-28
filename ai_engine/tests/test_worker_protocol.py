from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "src"


class WorkerProtocolTests(unittest.TestCase):
    def test_persistent_health_and_shutdown_roundtrip(self) -> None:
        env = os.environ.copy()
        env["PYTHONPATH"] = str(SRC)
        process = subprocess.Popen(
            [sys.executable, "-u", "-m", "retinacare_ai.worker"],
            cwd=ROOT,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        assert process.stdin is not None and process.stdout is not None
        try:
            for request_id, command in (("health-1", "health"), ("bye-1", "shutdown")):
                request = {
                    "protocol_version": 1,
                    "request_id": request_id,
                    "command": command,
                    "payload": {},
                }
                process.stdin.write(json.dumps(request) + "\n")
                process.stdin.flush()
                response = json.loads(process.stdout.readline())
                self.assertEqual(response["request_id"], request_id)
                self.assertEqual(response["status"], "ok")
                if command == "health":
                    self.assertIn("device", response["data"])
                    self.assertIn("model_state", response["data"])
                    self.assertIn("timings_ms", response)
            self.assertEqual(process.wait(timeout=5), 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            process.stdin.close()
            process.stdout.close()
            if process.stderr is not None:
                process.stderr.close()

    def test_unknown_command_does_not_claim_success(self) -> None:
        sys.path.insert(0, str(SRC))
        from retinacare_ai.worker import handle

        response = handle({
            "protocol_version": 1,
            "request_id": "bad-1",
            "command": "diagnose_everything",
            "payload": {},
        })
        self.assertEqual(response["status"], "error")
        self.assertEqual(response["error"]["code"], "unknown_command")

    def test_invalid_envelopes_are_rejected(self) -> None:
        sys.path.insert(0, str(SRC))
        from retinacare_ai.worker import handle

        cases = [
            ({"protocol_version": 2, "request_id": "v", "command": "health"}, "protocol_version_unsupported"),
            ({"protocol_version": 1, "request_id": "", "command": "health"}, "invalid_request"),
            ({"protocol_version": 1, "request_id": "p", "command": "health", "payload": []}, "invalid_payload"),
        ]
        for request, code in cases:
            with self.subTest(code=code):
                self.assertEqual(handle(request)["error"]["code"], code)

    def test_missing_model_never_returns_a_clinical_result(self) -> None:
        sys.path.insert(0, str(SRC))
        from PIL import Image
        import retinacare_ai.worker as worker

        with tempfile.TemporaryDirectory(prefix="RetinaCare space 患者-") as directory:
            worker.MODEL_DIR = Path(directory) / "models"
            worker.MODEL_DIR.mkdir()
            image_path = Path(directory) / "fundus image.png"
            Image.new("RGB", (8, 8), "black").save(image_path)
            response = worker.handle({
                "protocol_version": 1,
                "request_id": "missing-model",
                "command": "analyze_image",
                "payload": {"image_path": str(image_path)},
            })
            self.assertEqual(response["status"], "error")
            self.assertEqual(response["error"]["code"], "no_trained_weights")
            self.assertEqual(response["data"]["findings"], [])

    def test_oversized_request_is_rejected_and_worker_recovers(self) -> None:
        env = os.environ.copy()
        env["PYTHONPATH"] = str(SRC)
        process = subprocess.Popen(
            [sys.executable, "-u", "-m", "retinacare_ai.worker"],
            cwd=ROOT,
            env=env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        assert process.stdin is not None and process.stdout is not None
        try:
            process.stdin.write("x" * (1024 * 1024 + 1) + "\n")
            process.stdin.flush()
            self.assertEqual(json.loads(process.stdout.readline())["error"]["code"], "request_too_large")
            process.stdin.write(json.dumps({"protocol_version": 1, "request_id": "bye", "command": "shutdown", "payload": {}}) + "\n")
            process.stdin.flush()
            self.assertEqual(json.loads(process.stdout.readline())["status"], "ok")
            self.assertEqual(process.wait(timeout=5), 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)
            process.stdin.close()
            process.stdout.close()
            if process.stderr is not None:
                process.stderr.close()
