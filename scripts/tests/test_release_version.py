import unittest

from scripts.release_version import release_tag


class ReleaseVersionTests(unittest.TestCase):
    def test_first_major_release_uses_existing_go_module_path(self):
        self.assertEqual(release_tag("1.12.0", "1.12.0", "1.12.0"), "v1.12.0")

    def test_rejects_inconsistent_package_versions(self):
        with self.assertRaisesRegex(ValueError, "versions disagree"):
            release_tag("0.2.0", "0.1.0", "0.2.0")

    def test_rejects_versions_that_cannot_be_published_in_all_ecosystems(self):
        for version in ("2.0.0", "01.2.3", "1.0.0-rc.1"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                release_tag(version, version, version)
