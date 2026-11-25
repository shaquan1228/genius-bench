import argparse
import os
from pathlib import Path
import shutil
import sys
import tempfile


BINDING = Path(__file__).resolve().parent
ROOT = BINDING.parents[1]
REPOSITORY = "github.com/shaquan1228/genius-bench"
MODULE = f"{REPOSITORY}/bindings/go/genius-bench-kernel"
EXAMPLE = BINDING / "genius-bench-kernel/examples/grade_response/main.go"
sys.path.insert(0, str(ROOT / "scripts"))
from support import check_grading_example, run  # noqa: E402


def verify(version, repository):
    with tempfile.TemporaryDirectory(prefix="kernel-module-") as directory:
        temporary = Path(directory)
        project = temporary / "consumer"
        project.mkdir()
        shutil.copy(EXAMPLE, project / "main.go")
        environment = {
            **os.environ,
            "GOENV": "off",
            "GOWORK": "off",
            "GOFLAGS": "",
            "CGO_ENABLED": "1",
            "CGO_LDFLAGS": "",
            "GOMODCACHE": str(temporary / "modules"),
        }
        if repository:
            environment.update(
                GOPROXY="direct",
                GONOSUMDB=MODULE,
                GIT_CONFIG_COUNT="1",
                GIT_CONFIG_KEY_0=f"url.{repository.resolve().as_uri()}.insteadOf",
                GIT_CONFIG_VALUE_0=f"https://{REPOSITORY}",
            )
        run(
            ["go", "mod", "init", "example.com/kernel-consumer"],
            cwd=project,
            env=environment,
        )
        run(["go", "get", f"{MODULE}@{version}"], cwd=project, env=environment)
        check_grading_example(["go", "run", "."], cwd=project, env=environment)
        run(["go", "mod", "vendor"], cwd=project, env=environment)
        check_grading_example(
            ["go", "run", "-mod=vendor", "."],
            cwd=project,
            env={
                **environment,
                "GOPROXY": "off",
                "GOMODCACHE": str(temporary / "empty-modules"),
            },
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Grade from a downloaded Go module.")
    parser.add_argument("version")
    parser.add_argument("--repository", type=Path)
    args = parser.parse_args()
    verify(args.version, args.repository)
