import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "package_go_module.sh"
MODULE = "bindings/go/genius-bench-kernel"
VERSION = "v0.1.0"
TAG = f"refs/tags/{MODULE}/{VERSION}"


class GoModulePackageTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / "working copy"
        self.root.mkdir()
        self.bundle = Path(temporary.name) / "go.bundle"
        self.git("init", "--quiet")
        self.git("config", "user.name", "Test Author")
        self.git("config", "user.email", "author@example.org")
        module = self.root / MODULE
        module.mkdir(parents=True)
        (module / "go.mod").write_text("module example.org/kernel\n")
        self.git("add", MODULE)
        self.git("commit", "--quiet", "-m", "Release")
        self.release = self.git("rev-parse", "HEAD")
        self.archives = [
            module / f"libgenius_bench_go_{platform}.a"
            for platform in ("linux_amd64", "darwin_arm64")
        ]
        for archive in self.archives:
            archive.write_bytes(archive.name.encode())

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True).strip()

    def package(self, *args, check=True):
        return subprocess.run(
            ["bash", str(SCRIPT), MODULE, VERSION, str(self.bundle), *args],
            cwd=self.root,
            text=True,
            capture_output=True,
            check=check,
        )

    def test_package_contains_only_the_archives_and_reuses_the_release_tag(self):
        extra = self.root / MODULE / "libgenius_bench_go_old.a"
        extra.write_bytes(b"stale archive")
        self.git("add", str(extra))
        staged = self.git("write-tree")
        commit = self.package(VERSION).stdout.strip()
        self.assertEqual(self.git("rev-parse", "HEAD"), self.release)
        self.assertEqual(self.git("write-tree"), staged)
        self.assertEqual(self.git("tag", "--list"), "")
        self.git("fetch", "--quiet", str(self.bundle), f"{TAG}:{TAG}")
        self.assertEqual(self.git("rev-parse", TAG), commit)
        self.assertEqual(self.git("show", "-s", "--format=%P", TAG), self.release)
        self.assertEqual(
            set(self.git("diff", "--name-only", self.release, TAG).splitlines()),
            {archive.relative_to(self.root).as_posix() for archive in self.archives},
        )
        self.assertEqual(self.package(VERSION).stdout.strip(), commit)
        self.archives[0].write_bytes(b"different build")
        failed = self.package(VERSION, check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("differs", failed.stderr)
        self.package()
        self.assertEqual(self.git("rev-parse", TAG), commit)

    def test_release_tag_cannot_substitute_unrelated_files(self):
        (self.root / MODULE / "go.mod").write_text("module example.org/other\n")
        self.git("add", MODULE)
        self.git("commit", "--quiet", "-m", "Package with a changed manifest")
        self.git("tag", f"{MODULE}/{VERSION}")
        self.git("checkout", "--quiet", "--detach", self.release)
        for archive in self.archives:
            archive.write_bytes(archive.name.encode())
        failed = self.package(VERSION, check=False)
        self.assertNotEqual(failed.returncode, 0)
        self.assertIn("differs", failed.stderr)

    def publish(self, expected):
        remote = self.root.parent / "origin.git"
        subprocess.run(["git", "init", "--bare", "--quiet", str(remote)], check=True)
        self.git("remote", "add", "origin", str(remote))
        result = subprocess.run(
            ["bash", str(SCRIPT.with_name("update_go_module.sh")), "publish"],
            cwd=self.root,
            env={
                **os.environ,
                "MODULE_PATH": MODULE,
                "VERSION": VERSION,
                "MODULE_OBJECT": expected,
                "RUNNER_TEMP": str(self.bundle.parent),
            },
            text=True,
            capture_output=True,
        )
        return result, remote

    def test_publish_pushes_the_tested_object_from_the_bundle(self):
        commit = self.package().stdout.strip()
        result, remote = self.publish(commit)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git(f"--git-dir={remote}", "rev-parse", TAG), commit)

    def test_publish_rejects_an_object_other_than_the_tested_object(self):
        self.package()
        result, remote = self.publish(self.release)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("expected tested commit", result.stderr)
        self.assertEqual(self.git(f"--git-dir={remote}", "tag", "--list"), "")

    def test_package_accepts_module_and_bundle_paths_starting_with_dash(self):
        module = self.root / "-module"
        module.mkdir()
        (module / "go.mod").write_text("module example.org/kernel\n")
        self.git("add", "--", "-module/go.mod")
        self.git("commit", "--quiet", "-m", "Add another module")
        for platform in ("linux_amd64", "darwin_arm64"):
            (module / f"libgenius_bench_go_{platform}.a").write_bytes(b"archive")
        result = subprocess.run(
            ["bash", str(SCRIPT), module.name, VERSION, "-go.bundle"],
            cwd=self.root,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        tag = f"refs/tags/{module.name}/{VERSION}"
        self.git("fetch", "--quiet", str(self.root / "-go.bundle"), f"{tag}:{tag}")
        self.assertEqual(self.git("rev-parse", tag), result.stdout.strip())
