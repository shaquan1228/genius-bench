#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


REVISION = "523d1eede610171d825fde0fee12d418dd1ef981"
MODULES = ("failure", "support", "value", "predicate", "outcome", "assessment", "task")
ROOT = Path(__file__).resolve().parents[1]


def capture(source_repo, compiler):
    with tempfile.TemporaryDirectory(prefix="semantic-ocaml-") as directory:
        build = Path(directory)
        for module in MODULES:
            source = subprocess.check_output(
                ["git", "-C", str(source_repo), "show", f"{REVISION}:evals/ocaml/{module}.ml"]
            )
            (build / f"{module}.ml").write_bytes(source)
        shutil.copyfile(ROOT / "fixtures/export.ml", build / "export.ml")
        subprocess.run(
            [compiler, "-o", "export", *[f"{name}.ml" for name in MODULES], "export.ml"],
            cwd=build,
            check=True,
        )
        data = json.loads(subprocess.check_output([str(build / "export")], cwd=build))
        return {"source_revision": REVISION, "cases": data}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-repo", required=True, type=Path)
    parser.add_argument("--ocamlc", default="ocamlc")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    captured = capture(args.source_repo, args.ocamlc)
    cases = ",\n".join(
        "    " + json.dumps(case, ensure_ascii=False, separators=(",", ":"))
        for case in captured["cases"]
    )
    content = (
        "{\n  \"source_revision\": "
        + json.dumps(captured["source_revision"])
        + ",\n  \"cases\": [\n"
        + cases
        + "\n  ]\n}\n"
    )
    destination = ROOT / "fixtures/semantics.json"
    if args.check:
        if destination.read_text() != content:
            raise SystemExit("OCaml fixture capture differs from fixtures/semantics.json")
    else:
        destination.write_text(content)


if __name__ == "__main__":
    main()
