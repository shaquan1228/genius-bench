import hashlib
import io
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch
from urllib.error import HTTPError

from scripts.stage_release import stage_release

VERSION = "0.1.0"
CRATE = "genius-bench-kernel-0.1.0.crate"
WHEEL = "genius_bench_kernel-0.1.0-cp312-abi3-macosx_11_0_arm64.whl"
SDIST = "genius_bench_kernel-0.1.0.tar.gz"
SOURCE_GEM = "genius-bench-kernel-0.1.0.gem"
ARM_GEM = "genius-bench-kernel-0.1.0-arm64-darwin.gem"
LINUX_GEM = "genius-bench-kernel-0.1.0-x86_64-linux-gnu.gem"


def digest(name):
    return hashlib.sha256(name.encode()).hexdigest()


class StageReleaseTests(unittest.TestCase):
    def setUp(self):
        directory = TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        self.artifacts = Path(directory.name) / "artifacts"
        self.artifacts.mkdir()
        self.pending = Path(directory.name) / "pending"
        mock = patch("scripts.stage_release.urlopen")
        self.urlopen = mock.start()
        self.addCleanup(mock.stop)

    def build(self, *names):
        for name in names:
            (self.artifacts / name).write_bytes(name.encode())

    def registry(self, response):
        self.urlopen.return_value = io.BytesIO(json.dumps(response).encode())

    def stage(self, registry):
        return stage_release(registry, VERSION, self.artifacts, self.pending)

    def test_first_release_stages_all_files_after_a_registry_404(self):
        self.build(WHEEL, SDIST)
        self.urlopen.side_effect = HTTPError("", 404, "Not Found", {}, None)
        self.assertTrue(self.stage("pypi"))
        for name in (WHEEL, SDIST):
            self.assertEqual(
                (self.pending / name).read_bytes(), (self.artifacts / name).read_bytes()
            )

    def test_partial_pypi_release_stages_only_the_source_distribution(self):
        self.build(WHEEL, SDIST)
        self.registry(
            {
                "urls": [
                    {
                        "filename": WHEEL,
                        "yanked": False,
                        "digests": {"sha256": digest(WHEEL)},
                    }
                ]
            }
        )
        self.assertTrue(self.stage("pypi"))
        self.assertEqual(list(self.pending.iterdir()), [self.pending / SDIST])

    def test_gems_match_both_version_and_platform(self):
        self.build(SOURCE_GEM, ARM_GEM, LINUX_GEM)
        self.registry(
            [
                {"number": VERSION, "platform": "ruby", "sha": digest(SOURCE_GEM)},
                {"number": VERSION, "platform": "arm64-darwin", "sha": digest(ARM_GEM)},
                {"number": "0.0.9", "platform": "x86_64-linux-gnu", "sha": "other"},
            ]
        )
        self.assertTrue(self.stage("rubygems"))
        self.assertEqual(list(self.pending.iterdir()), [self.pending / LINUX_GEM])

    def test_matching_crate_skips_publication(self):
        self.build(CRATE)
        self.registry(
            {"version": {"num": VERSION, "checksum": digest(CRATE), "yanked": False}}
        )
        self.assertFalse(self.stage("crates-io"))
        self.assertEqual(list(self.pending.iterdir()), [])

    def test_a_yanked_crate_stops_publication_even_when_the_checksum_matches(self):
        self.build(CRATE)
        self.registry(
            {"version": {"num": VERSION, "checksum": digest(CRATE), "yanked": True}}
        )
        with self.assertRaisesRegex(ValueError, "yanked"):
            self.stage("crates-io")
        self.assertFalse(self.pending.exists())

    def test_a_digest_conflict_preserves_inputs_and_creates_no_staging_directory(self):
        self.build(SOURCE_GEM, ARM_GEM, LINUX_GEM)
        self.registry(
            [
                {"number": VERSION, "platform": "arm64-darwin", "sha": digest(ARM_GEM)},
                {"number": VERSION, "platform": "ruby", "sha": "different"},
            ]
        )
        with self.assertRaisesRegex(ValueError, "different"):
            self.stage("rubygems")
        self.assertFalse(self.pending.exists())
        for name in (SOURCE_GEM, ARM_GEM, LINUX_GEM):
            self.assertEqual((self.artifacts / name).read_bytes(), name.encode())

    def test_permission_and_service_errors_stop_publication(self):
        self.build(WHEEL)
        for status in (403, 503):
            with self.subTest(status=status):
                self.urlopen.side_effect = HTTPError("", status, "failed", {}, None)
                with self.assertRaises(HTTPError):
                    self.stage("pypi")
                self.assertFalse(self.pending.exists())

    def test_wrong_version_never_reaches_the_registry(self):
        self.build("genius-bench-kernel-0.0.9.crate")
        with self.assertRaisesRegex(ValueError, "Unexpected"):
            self.stage("crates-io")
        self.urlopen.assert_not_called()
