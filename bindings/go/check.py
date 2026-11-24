import argparse
import os
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/go"
sys.path.insert(0, str(ROOT / "scripts"))
from support import check_grading_example  # noqa: E402


def main():
    parser = argparse.ArgumentParser(
        description="Build and test the Go grading bridge."
    )
    parser.add_argument("--cargo", default=os.environ.get("CARGO", "cargo"))
    args = parser.parse_args()
    target = (ROOT / os.environ.get("CARGO_TARGET_DIR", "target")).resolve()
    manifest = ["--manifest-path", str(PACKAGE / "native/Cargo.toml")]
    build = ["--locked", "--offline", "--target-dir", str(target), *manifest]
    for command in [
        ["fmt", *manifest, "--", "--check"],
        ["clippy", *build, "--all-targets", "--", "-D", "warnings"],
        ["test", *build],
        ["build", "--release", *build],
    ]:
        subprocess.run([args.cargo, *command], cwd=ROOT, check=True)
    environment = dict(os.environ, CGO_ENABLED="1")
    search = f'"-L{target / "release"}"'
    environment["CGO_LDFLAGS"] = (
        f"{search} {environment.get('CGO_LDFLAGS', '')}".strip()
    )
    subprocess.run(
        ["go", "test", "-a", "-count=1", "./..."],
        cwd=PACKAGE,
        env=environment,
        check=True,
    )

    check_grading_example(
        ["go", "run", "-a", "./examples/grade_response"], cwd=PACKAGE, env=environment
    )


if __name__ == "__main__":
    main()
