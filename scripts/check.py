#!/usr/bin/env python3
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]


def git(*arguments, input_text=None):
    return subprocess.check_output(
        ["git", *arguments], cwd=ROOT, input=input_text, text=True
    ).splitlines()


def pushed_code_tips(remote, updates):
    tips = []
    for update in updates:
        _, local, _, previous = update.split()
        if set(local) == {"0"}:
            continue
        paths = set()
        if set(previous) == {"0"}:
            commits = git("rev-list", local, "--not", f"--remotes={remote}")
            if commits:
                paths.update(
                    git(
                        "diff-tree",
                        "--root",
                        "-m",
                        "--no-commit-id",
                        "--name-only",
                        "-r",
                        "--stdin",
                        input_text="\n".join(commits) + "\n",
                    )
                )
        else:
            paths.update(git("diff", "--name-only", previous, local))
        if (
            any(not path.endswith(".md") and path != "LICENSE" for path in paths)
            and local not in tips
        ):
            tips.append(local)
    return tips


def check_pushed_tips(tips, cargo):
    executable = shutil.which(cargo)
    if executable is None:
        raise ValueError(f"Cargo executable not found: {cargo}")
    executable = os.path.abspath(executable)
    environment = dict(os.environ)
    for variable in git("rev-parse", "--local-env-vars"):
        environment.pop(variable, None)
    for tip in tips:
        with tempfile.TemporaryDirectory(prefix="semantic-core-push-") as temporary:
            directory = Path(temporary)
            snapshot = directory / "repository"
            subprocess.run(
                [
                    "git",
                    "clone",
                    "--no-hardlinks",
                    "--no-checkout",
                    "--quiet",
                    str(ROOT),
                    str(snapshot),
                ],
                env=environment,
                check=True,
            )
            subprocess.run(
                ["git", "checkout", "--detach", "--quiet", tip],
                cwd=snapshot,
                env=environment,
                check=True,
            )
            environment["CARGO_TARGET_DIR"] = str(directory / "target")
            print(f"Checking pushed commit {tip}", flush=True)
            subprocess.run(
                [sys.executable, "scripts/check.py", "--cargo", executable],
                cwd=snapshot,
                env=environment,
                check=True,
            )


def check(cargo):
    commands = [
        [sys.executable, "-m", "unittest", "discover", "-s", "scripts/tests"],
        [cargo, "test", "--locked", "--offline", "-p", "semantic-core"],
        [cargo, "fmt", "--all", "--", "--check"],
        [cargo, "clippy", "--locked", "--offline", "-p", "semantic-core", "--all-targets", "--", "-D", "warnings"],
    ]
    environment = dict(os.environ)
    environment.setdefault("CARGO_BUILD_JOBS", "2")
    for command in commands:
        print("+ " + " ".join(command), flush=True)
        subprocess.run(command, cwd=ROOT, env=environment, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cargo", default=os.environ.get("CARGO", "cargo"))
    parser.add_argument("--pre-push", nargs=2, metavar=("REMOTE", "URL"))
    args = parser.parse_args()
    if args.pre_push:
        tips = pushed_code_tips(args.pre_push[0], sys.stdin)
        if not tips:
            print("Documentation-only push: core checks skipped.")
            return
        check_pushed_tips(tips, args.cargo)
        return
    check(args.cargo)


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, ValueError) as error:
        raise SystemExit(str(error)) from error
