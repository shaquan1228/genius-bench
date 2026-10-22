import argparse
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile

from verify_wheel import source_identity, verify


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/python"


def run(command, *, cwd=ROOT, env=None):
    print("+ " + " ".join(map(str, command)), flush=True)
    subprocess.run(command, cwd=cwd, env=env, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--uv", default=os.environ.get("UV", "uv"))
    parser.add_argument("--python", default="3.12")
    args = parser.parse_args()
    run(
        [
            args.uv,
            "sync",
            "--project",
            str(PACKAGE),
            "--locked",
            "--offline",
            "--no-install-project",
            "--python",
            args.python,
        ]
    )
    python = PACKAGE / ".venv/bin/python"
    run(
        [
            str(python),
            "-m",
            "ruff",
            "format",
            "--check",
            "scripts/check.py",
            str(PACKAGE),
        ]
    )
    with tempfile.TemporaryDirectory(prefix="kernel-source-") as directory:
        temporary = Path(directory)
        run(
            [
                str(python),
                "-m",
                "maturin",
                "sdist",
                "--manifest-path",
                str(PACKAGE / "Cargo.toml"),
                "--out",
                str(temporary),
            ]
        )
        source = next(temporary.glob("*.tar.gz"))
        with tarfile.open(source) as archive:
            if any(
                name.endswith(".so") or "__pycache__" in name
                for name in archive.getnames()
            ):
                raise RuntimeError(
                    "Source distribution contains compiled Python artifacts"
                )
            archive.extractall(temporary / "source", filter="data")
        extracted = next((temporary / "source").iterdir())
        for expected in (
            "Cargo.lock",
            "crates/semantic-core/build.rs",
            "crates/semantic-core/src/lib.rs",
            "bindings/python/src/lib.rs",
            "python/kernel/__init__.pyi",
            "python/kernel/py.typed",
        ):
            if not (extracted / expected).is_file():
                raise RuntimeError(f"Source distribution missing {expected}")
        core = ROOT / "crates/semantic-core"
        packaged_core = extracted / "crates/semantic-core"
        for source in [core / "build.rs", *sorted((core / "src").glob("*.rs"))]:
            relative = source.relative_to(core)
            if source.read_bytes() != (packaged_core / relative).read_bytes():
                raise RuntimeError(
                    f"Source distribution changed core source {relative}"
                )
        if (packaged_core / "LICENSE").read_bytes() != (ROOT / "LICENSE").read_bytes():
            raise RuntimeError("Source distribution changed the core license")
        expected_manifest = (
            (core / "Cargo.toml")
            .read_text()
            .replace("license-file.workspace = true", 'license-file = "LICENSE"')
        )
        if (packaged_core / "Cargo.toml").read_text() != expected_manifest:
            raise RuntimeError("Unexpected core manifest normalization")
        run(
            [
                str(python),
                "-m",
                "maturin",
                "build",
                "--release",
                "--locked",
                "--offline",
                "--interpreter",
                str(python),
                "--out",
                str(temporary / "wheels"),
            ],
            cwd=extracted,
            env={**os.environ, "CARGO_TARGET_DIR": str(temporary / "target")},
        )
        wheel = next((temporary / "wheels").glob("*.whl"))
        verify(wheel, str(python), args.uv, True, source_identity(packaged_core))


if __name__ == "__main__":
    main()
