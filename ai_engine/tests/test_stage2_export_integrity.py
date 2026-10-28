import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))

from scripts import stage2_export_lesion_parity


class Stage2CheckpointIntegrityTests(unittest.TestCase):
    def test_wrong_size_is_rejected_before_deserialization(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidate.pth"
            path.write_bytes(b"not a checkpoint")
            with self.assertRaisesRegex(ValueError, "size mismatch"):
                stage2_export_lesion_parity.verify_checkpoint(path)

    def test_wrong_digest_is_rejected_even_when_size_matches(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidate.pth"
            path.write_bytes(b"abc")
            with (
                patch.dict(
                    stage2_export_lesion_parity.MODEL_SPECS,
                    {"MA": {"size": 3, "sha256": "0" * 64, "threshold": 0.01, "task": "test"}},
                ),
                self.assertRaisesRegex(ValueError, "SHA-256 mismatch"),
            ):
                stage2_export_lesion_parity.verify_checkpoint(path)

    def test_he_checkpoint_metadata_is_independent_from_ma(self):
        self.assertEqual(
            stage2_export_lesion_parity.checkpoint_name("HE"),
            "stage2_hsmoe_aunet_HE_best_model.pth",
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "candidate.pth"
            path.write_bytes(b"abc")
            with patch.dict(
                stage2_export_lesion_parity.MODEL_SPECS,
                {"HE": {"size": 3, "sha256": "0" * 64, "threshold": 0.2, "task": "test"}},
            ), self.assertRaisesRegex(ValueError, "SHA-256 mismatch"):
                stage2_export_lesion_parity.verify_checkpoint(path, "HE")


if __name__ == "__main__":
    unittest.main()
