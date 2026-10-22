from pathlib import Path
import os
import subprocess
import sys
import tempfile
import unittest


DRIVER = Path(__file__).resolve().parents[1] / "check.py"
ZERO = "0" * 40
VALIDATOR = """from pathlib import Path
import os
import sys

verdict = Path("verdict.txt").read_text().strip()
print(f"validated:{verdict}", flush=True)
with open(os.environ["PUSH_GATE_RECORD"], "a") as record:
    record.write(verdict + "\\n")
raise SystemExit(0 if verdict == "good" else 1)
"""


class PushGateTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name)
        self.repository = self.directory / "repository"
        self.repository.mkdir()
        self.record = self.directory / "record.txt"
        self.cargo = self.directory / "cargo"
        self.cargo.write_text(f"#!{sys.executable}\n" + VALIDATOR)
        self.cargo.chmod(0o755)
        self.git("init", "--quiet", "--initial-branch=main")
        self.git("config", "user.name", "Test Maintainer")
        self.git("config", "user.email", "maintainer@example.com")
        (self.repository / "scripts").mkdir()
        (self.repository / "scripts/check.py").write_text(VALIDATOR)
        self.base = self.commit("good", "base")
        self.git("update-ref", "refs/remotes/origin/main", self.base)

    def git(self, *arguments):
        result = subprocess.run(
            ["git", *arguments],
            cwd=self.repository,
            text=True,
            capture_output=True,
            check=True,
        )
        return result.stdout.strip()

    def commit(self, verdict, source):
        (self.repository / "verdict.txt").write_text(verdict)
        (self.repository / "probe.py").write_text(source)
        self.git("add", "scripts/check.py", "verdict.txt", "probe.py")
        self.git("commit", "--quiet", "-m", source)
        return self.git("rev-parse", "HEAD")

    def update(self, tip, previous, name="main"):
        return f"refs/heads/{name} {tip} refs/heads/{name} {previous}\n"

    def push(self, updates, git_environment=False):
        (self.repository / "scripts/check.py").write_text(DRIVER.read_text())
        hook = self.repository / ".git/hooks/pre-push"
        hook.write_text(
            f"#!{sys.executable}\n"
            "from pathlib import Path\n"
            "import os\n"
            "import sys\n"
            "driver = Path.cwd() / 'scripts/check.py'\n"
            "os.execv(sys.executable, [sys.executable, str(driver), "
            "'--pre-push', *sys.argv[1:]])\n"
        )
        hook.chmod(0o755)
        environment = dict(os.environ)
        environment.update(
            CARGO=str(self.cargo),
            PUSH_GATE_RECORD=str(self.record),
        )
        if git_environment:
            environment.update(
                GIT_DIR=str(self.repository / ".git"),
                GIT_WORK_TREE=str(self.repository),
            )
        return subprocess.run(
            [str(hook), "origin", "unused"],
            cwd=self.repository,
            env=environment,
            input=updates,
            text=True,
            capture_output=True,
        )

    def recorded(self):
        return self.record.read_text().splitlines() if self.record.exists() else []

    def test_rejects_bad_committed_tip_despite_uncommitted_repair(self):
        bad = self.commit("bad", "broken")
        (self.repository / "verdict.txt").write_text("good")
        result = self.push(self.update(bad, self.base))
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["bad"])

    def test_checks_other_branch_tip_instead_of_current_bad_tree(self):
        good = self.commit("good", "working")
        self.commit("bad", "broken")
        result = self.push(self.update(good, self.base, "working"))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["good"])

    def test_isolates_repository_environment_exported_by_git(self):
        good = self.commit("good", "working")
        self.commit("bad", "broken")
        result = self.push(
            self.update(good, self.base, "working"), git_environment=True
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["good"])
        self.assertEqual((self.repository / "verdict.txt").read_text(), "bad")

    def test_checks_each_distinct_code_tip(self):
        good = self.commit("good", "working")
        bad = self.commit("bad", "broken")
        (self.repository / "verdict.txt").write_text("good")
        result = self.push(
            self.update(good, self.base, "working")
            + self.update(bad, self.base, "broken")
        )
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["good", "bad"])

    def test_deduplicates_code_tips(self):
        good = self.commit("good", "working")
        result = self.push(
            self.update(good, self.base, "first")
            + self.update(good, self.base, "second")
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["good"])

    def test_skips_documentation_tip_without_a_validator_in_mixed_push(self):
        good = self.commit("good", "working")
        self.git("checkout", "--quiet", "--orphan", "docs")
        self.git("rm", "--quiet", "-rf", "scripts/check.py", "verdict.txt", "probe.py")
        (self.repository / "README.md").write_text("Documentation\n")
        self.git("add", "README.md")
        self.git("commit", "--quiet", "-m", "Document project")
        docs = self.git("rev-parse", "HEAD")
        self.git("checkout", "--quiet", "main")
        result = self.push(
            self.update(docs, ZERO, "docs")
            + self.update(good, self.base, "working")
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), ["good"])

    def test_checks_source_renamed_as_documentation(self):
        self.git("config", "diff.renames", "true")
        self.git("mv", "probe.py", "probe.md")
        self.git("commit", "--quiet", "-m", "Rename source as documentation")
        renamed = self.git("rev-parse", "HEAD")
        for previous in (self.base, ZERO):
            with self.subTest(previous=previous):
                self.record.unlink(missing_ok=True)
                result = self.push(self.update(renamed, previous, "renamed"))
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(self.recorded(), ["good"])

    def test_skips_documentation_updates_and_deleted_refs(self):
        (self.repository / "README.md").write_text("Documentation\n")
        self.git("add", "README.md")
        self.git("commit", "--quiet", "-m", "Document project")
        docs = self.git("rev-parse", "HEAD")
        result = self.push(
            self.update(docs, self.base, "docs")
            + self.update(ZERO, self.base, "deleted")
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.recorded(), [])


if __name__ == "__main__":
    unittest.main()
