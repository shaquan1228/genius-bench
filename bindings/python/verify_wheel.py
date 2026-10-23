import argparse
import os
from pathlib import Path
import shutil
import sys
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/python"
sys.path.insert(0, str(ROOT / "scripts"))
from support import run, source_identity  # noqa: E402


def verify(wheel, python, uv, offline, expected_source_hash=None):
    with zipfile.ZipFile(wheel) as archive:
        for name in ("kernel/__init__.pyi", "kernel/py.typed"):
            if name not in archive.namelist():
                raise RuntimeError(f"Wheel missing {name}")
        if not any(
            name.startswith("kernel/_native.") and name.endswith((".so", ".pyd"))
            for name in archive.namelist()
        ):
            raise RuntimeError("Wheel missing native extension")
    with tempfile.TemporaryDirectory(prefix="kernel-wheel-") as directory:
        temporary = Path(directory)
        environment = temporary / "environment"
        run([uv, "venv", str(environment), "--python", python], cwd=temporary)
        installed_python = environment / "bin/python"
        install = [uv, "pip", "install", "--python", str(installed_python)]
        if offline:
            install.append("--offline")
        shutil.copytree(
            PACKAGE / "tests",
            temporary / "tests",
            ignore=shutil.ignore_patterns("__pycache__"),
        )
        empty_path = temporary / "empty-path"
        empty_path.mkdir()
        clean = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("PYTHON")
            and key not in {"VIRTUAL_ENV", "CONDA_PREFIX"}
        }
        clean["PATH"] = str(empty_path)
        clean["KERNEL_SOURCE_HASH"] = expected_source_hash or source_identity()
        clean["KERNEL_COMPILER_FREE"] = "1"
        clean["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
        run([*install, str(wheel)], cwd=temporary)
        run(
            [str(installed_python), "-I", "tests/smoke.py"],
            cwd=temporary,
            env=clean,
        )
        requirements = temporary / "pylock.toml"
        export = [
            uv,
            "export",
            "--project",
            str(PACKAGE),
            "--locked",
            "--only-group",
            "dev",
            "--no-emit-project",
            "--no-emit-package",
            "maturin",
            "--no-emit-package",
            "ruff",
            "--no-header",
            "--no-annotate",
            "--output-file",
            str(requirements),
        ]
        if offline:
            export.append("--offline")
        run(export, cwd=temporary, capture=True)
        run(
            [*install, "--require-hashes", "--requirements", str(requirements)],
            cwd=temporary,
        )
        run(
            [str(installed_python), "-I", "-m", "pytest", "tests", "-q"],
            cwd=temporary,
            env=clean,
        )
        run(
            [
                str(installed_python),
                "-I",
                "-m",
                "mypy",
                "--strict",
                "tests/typing_valid.py",
            ],
            cwd=temporary,
            env=clean,
        )
        run(
            [str(installed_python), "-I", "-m", "mypy.stubtest", "kernel", "--concise"],
            cwd=temporary,
            env=clean,
        )
        invalid = run(
            [
                str(installed_python),
                "-I",
                "-m",
                "mypy",
                "--strict",
                "tests/typing_invalid.py",
            ],
            cwd=temporary,
            env=clean,
            capture=True,
            check=False,
        )
        if (
            invalid.returncode != 1
            or "Found 4 errors" not in invalid.stdout
            or any(
                code not in invalid.stdout
                for code in ("[call-arg]", "[misc]", "[attr-defined]")
            )
        ):
            raise RuntimeError(
                f"Unexpected negative typing result:\n{invalid.stdout}\n{invalid.stderr}"
            )
        print(
            "Installed wheel: boundary, identity, stubs, and positive/negative typing passed.",
            flush=True,
        )


def main():
    parser = argparse.ArgumentParser(
        description="Install a built wheel into a fresh environment and verify it."
    )
    parser.add_argument("--wheel", required=True, type=Path)
    parser.add_argument("--python", default=sys.executable)
    parser.add_argument("--uv", default=os.environ.get("UV", "uv"))
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    verify(
        args.wheel.resolve(),
        str(Path(args.python).resolve()) if "/" in args.python else args.python,
        args.uv,
        args.offline,
    )


if __name__ == "__main__":
    main()
