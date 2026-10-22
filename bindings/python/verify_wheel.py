import argparse
import hashlib
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/python"


def run(command, *, cwd, env=None, capture=False):
    print("+ " + " ".join(map(str, command)), flush=True)
    return subprocess.run(
        command, cwd=cwd, env=env, text=True, capture_output=capture, check=not capture
    )


def source_identity(core=None):
    core = core or ROOT / "crates/semantic-core"
    build = (core / "build.rs").read_text()
    names = re.findall(
        r'"([^"]+)"', build.split("pub const SOURCE_INPUTS:")[1].split("];", 1)[0]
    )
    digest = hashlib.sha256()
    for name in names:
        contents = (core / name).read_bytes()
        digest.update(len(name).to_bytes(8, "little"))
        digest.update(name.encode())
        digest.update(len(contents).to_bytes(8, "little"))
        digest.update(contents)
    return digest.hexdigest()


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
        shutil.copyfile(ROOT / "fixtures/semantics.json", temporary / "semantics.json")
        empty_path = temporary / "empty-path"
        empty_path.mkdir()
        clean = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith("PYTHON")
            and key not in {"VIRTUAL_ENV", "CONDA_PREFIX"}
        }
        clean["PATH"] = str(empty_path)
        clean["KERNEL_FIXTURES"] = str(temporary / "semantics.json")
        clean["KERNEL_SOURCE_HASH"] = expected_source_hash or source_identity()
        clean["KERNEL_COMPILER_FREE"] = "1"
        clean["PYTEST_DISABLE_PLUGIN_AUTOLOAD"] = "1"
        run([*install, str(wheel)], cwd=temporary)
        run(
            [str(installed_python), "-I", "tests/smoke.py"],
            cwd=temporary,
            env=clean,
        )
        requirements = temporary / "requirements.txt"
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
        run(export, cwd=temporary, capture=True).check_returncode()
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
            "Installed wheel: fixtures, boundary, identity, stubs, and positive/negative typing passed.",
            flush=True,
        )


def main():
    parser = argparse.ArgumentParser()
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
