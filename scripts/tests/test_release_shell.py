import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]


class ReleaseShellTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="release checks ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.pending = self.root / "pending artifacts"
        self.pending.mkdir()
        self.log = self.root / "commands.jsonl"
        self.environment = {
            **os.environ,
            "PATH": f"{self.bin}{os.pathsep}{os.environ['PATH']}",
            "COMMAND_LOG": str(self.log),
        }

    def tool(self, name, body):
        path = self.bin / name
        path.write_text(
            "#!/usr/bin/env python3\n"
            "import json, os, sys\n"
            "from pathlib import Path\n"
            "with open(os.environ['COMMAND_LOG'], 'a') as output:\n"
            "    output.write(json.dumps([os.getcwd(), sys.argv[1:]]) + '\\n')\n"
            + body
        )
        path.chmod(0o755)

    def run_script(self, name, *arguments):
        return subprocess.run(
            ["bash", str(SCRIPTS / name), *map(str, arguments)],
            cwd=self.root,
            env=self.environment,
            capture_output=True,
            text=True,
        )

    def commands(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def crate(self, contents):
        (self.pending / "kernel.crate").write_bytes(b"tested")
        self.tool(
            "cargo",
            "if sys.argv[1] == 'package':\n"
            "    directory = Path('target/package')\n"
            "    directory.mkdir(parents=True)\n"
            f"    (directory / 'kernel.crate').write_bytes({contents!r})\n",
        )

    def test_publication_requires_the_rebuilt_crate_to_match(self):
        self.crate(b"different")
        result = self.run_script("publish_release.sh", "crates-io", self.pending)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to publish", result.stderr)
        self.assertFalse(any(args[0] == "publish" for _, args in self.commands()))

    def test_publication_accepts_an_identical_crate(self):
        self.crate(b"tested")
        result = self.run_script("publish_release.sh", "crates-io", self.pending)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.commands()[-1][1][0], "publish")

    def test_publication_rejects_missing_or_ambiguous_crates(self):
        self.tool("cargo", "sys.exit('Cargo must not run')\n")
        for names in ((), ("one.crate", "two.crate")):
            with self.subTest(names=names):
                for name in names:
                    (self.pending / name).write_bytes(b"crate")
                result = self.run_script("publish_release.sh", "crates-io", self.pending)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Expected one staged crate", result.stderr)
                self.assertFalse(self.log.exists())

    def test_gem_publication_preserves_paths_with_spaces_and_ignores_other_files(self):
        self.tool("gem", "")
        gem = self.pending / "native package.gem"
        gem.write_bytes(b"gem")
        (self.pending / "unrelated.txt").write_text("not a package")
        result = self.run_script("publish_release.sh", "rubygems", self.pending)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([args for _, args in self.commands()], [["push", "--", str(gem)]])

    def test_gem_publication_rejects_an_empty_directory(self):
        result = self.run_script("publish_release.sh", "rubygems", self.pending)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("No staged gems", result.stderr)

    def test_local_python_install_rejects_missing_or_ambiguous_wheels(self):
        for names in ((), ("one.whl", "two.whl")):
            with self.subTest(names=names):
                for name in names:
                    (self.pending / name).write_bytes(b"wheel")
                result = self.run_script("verify_install.sh", "--local", "python", self.pending)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("Expected one wheel", result.stderr)

    def test_local_rust_consumer_checks_scores_and_removes_its_workspace(self):
        self.tool(
            "cargo",
            "if sys.argv[1] == 'init':\n"
            "    Path('src').mkdir()\n"
            "if sys.argv[1] == 'run':\n"
            "    print('correctness true\\nscope false')\n",
        )
        version = subprocess.check_output(
            ["python3", str(SCRIPTS / "release_version.py")], text=True
        ).strip().removeprefix("v")
        package = self.pending / f"genius-bench-kernel-{version}"
        package.mkdir()
        (package / "Cargo.toml").touch()
        for arguments in (
            ("--local", "rust", self.pending),
            ("rust", version, package),
        ):
            with self.subTest(arguments=arguments):
                result = self.run_script("verify_install.sh", *arguments)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(
                    ["add", "--path", str(package)],
                    [args for _, args in self.commands()],
                )
                self.assertTrue(all(not Path(cwd).exists() for cwd, _ in self.commands()))

    def test_consumer_rejects_changed_scores(self):
        self.tool(
            "cargo",
            "if sys.argv[1] == 'init':\n"
            "    Path('src').mkdir()\n"
            "if sys.argv[1] == 'run':\n"
            "    print('correctness true\\nscope true')\n",
        )
        result = self.run_script("verify_install.sh", "rust", "0.1.0")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("+scope true", result.stdout)
        self.assertTrue(all(not Path(cwd).exists() for cwd, _ in self.commands()))

    def test_staging_failure_does_not_report_a_successful_skip(self):
        helper = self.root / "scripts/stage_release.py"
        helper.parent.mkdir()
        helper.write_text("raise RuntimeError('registry unavailable')\n")
        output = self.root / "step-output"
        output.touch()
        self.environment.update(GITHUB_REF_NAME="v0.1.0", GITHUB_OUTPUT=str(output))
        result = self.run_script("stage_release.sh", "pypi")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(output.read_text(), "")

    def test_relative_script_ignores_cdpath_when_finding_repository(self):
        (self.root / "scripts").mkdir()
        self.environment["CDPATH"] = str(self.root)
        self.tool(
            "cargo",
            "if sys.argv[1] == 'init':\n"
            "    Path('src').mkdir()\n"
            "if sys.argv[1] == 'run':\n"
            "    print('correctness true\\nscope false')\n",
        )
        result = subprocess.run(
            ["bash", "scripts/verify_install.sh", "rust", "0.1.0"],
            cwd=SCRIPTS.parent,
            env=self.environment,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_gem_publication_accepts_a_relative_directory_starting_with_dash(self):
        directory = self.root / "-pending gems"
        directory.mkdir()
        (directory / "native.gem").write_bytes(b"gem")
        self.tool("gem", "")
        result = self.run_script("publish_release.sh", "rubygems", directory.name)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            [args for _, args in self.commands()],
            [["push", "--", "-pending gems/native.gem"]],
        )
