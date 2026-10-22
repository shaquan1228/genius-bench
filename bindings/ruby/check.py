import argparse
import os
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/ruby"


def run(command, *, cwd=PACKAGE, env=None):
    print("+ " + " ".join(map(str, command)), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--bundle", default=os.environ.get("BUNDLE", "bundle"))
    parser.add_argument("--ruby", default=os.environ.get("RUBY", "ruby"))
    parser.add_argument("--cargo", default=os.environ.get("CARGO", "cargo"))
    args = parser.parse_args()
    environment = {
        **os.environ,
        "BUNDLE_GEMFILE": str(PACKAGE / "Gemfile"),
        "BUNDLE_FROZEN": "true",
    }
    run([args.cargo, "fmt", "--all", "--", "--check"], env=environment)
    run(
        [
            args.cargo,
            "clippy",
            "--locked",
            "--offline",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        env=environment,
    )
    run([args.bundle, "exec", "rake", "compile", "spec"], env=environment)


if __name__ == "__main__":
    main()
