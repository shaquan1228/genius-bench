import hashlib
import io
import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from scripts.pending_wheels import pending


def published_file(filename, digest):
    return io.BytesIO(
        json.dumps(
            {"urls": [{"filename": filename, "digests": {"sha256": digest}}]}
        ).encode()
    )


class PendingWheelsTests(unittest.TestCase):
    def test_retry_uploads_only_missing_files(self):
        with TemporaryDirectory() as directory:
            artifacts = Path(directory)
            published = artifacts / "uploaded.whl"
            published.write_bytes(b"wheel")
            missing = artifacts / "pending.tar.gz"
            missing.write_bytes(b"source")
            digest = hashlib.sha256(b"wheel").hexdigest()
            with patch(
                "scripts.pending_wheels.urlopen",
                return_value=published_file(published.name, digest),
            ):
                self.assertTrue(pending("0.1.0", artifacts))
            self.assertEqual(list(artifacts.iterdir()), [missing])

    def test_retry_rejects_a_different_published_file(self):
        with TemporaryDirectory() as directory:
            artifacts = Path(directory)
            wheel = artifacts / "uploaded.whl"
            wheel.write_bytes(b"wheel")
            with patch(
                "scripts.pending_wheels.urlopen",
                return_value=published_file(wheel.name, "different"),
            ):
                with self.assertRaisesRegex(ValueError, wheel.name):
                    pending("0.1.0", artifacts)
            self.assertTrue(wheel.exists())
