#!/usr/bin/env python3
import argparse
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]


def git(*arguments, input_text=None):
    return subprocess.check_output(
        ["git", *arguments], cwd=ROOT, input=input_text, text=True
    ).splitlines()


def pushed_paths(remote, updates):
    paths = set()
    for update in updates:
        _, local, _, previous = update.split()
        if set(local) == {"0"}:
            continue
        if set(previous) == {"0"}:
            commits = git("rev-list", local, "--not", f"--remotes={remote}")
            if commits:
                paths.update(git("diff-tree", "--root", "-m", "--no-commit-id", "--name-only", "-r", "--stdin", input_text="\n".join(commits) + "\n"))
        else:
            paths.update(git("diff", "--name-only", previous, local))
    return paths


def check(cargo):
    commands = [
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
        paths = pushed_paths(args.pre_push[0], sys.stdin)
        if all(path.endswith(".md") or path == "LICENSE" for path in paths):
            print("Documentation-only push: core checks skipped.")
            return
    check(args.cargo)


if __name__ == "__main__":
    try:
        main()
    except (subprocess.CalledProcessError, ValueError) as error:
        raise SystemExit(str(error)) from error
