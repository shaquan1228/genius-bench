import argparse
import os
from pathlib import Path
import shutil
import sys
import tomllib

from verify_gem import source_identity, verify


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/ruby"
CORE = "crates/semantic-core"
sys.path.insert(0, str(ROOT / "scripts"))
from support import run as run_in  # noqa: E402


def run(command, *, cwd=PACKAGE, env=None):
    run_in(command, cwd=cwd, env=env)


def packages(lock):
    return {
        (package["name"], package["version"]): package.get("checksum")
        for package in tomllib.loads(lock.read_text())["package"]
    }


def check_stage(stage):
    core = ROOT / CORE
    staged_core = stage / CORE
    sources = [
        core / "Cargo.toml",
        core / "build.rs",
        *sorted((core / "src").rglob("*")),
    ]
    staged = sorted(path for path in staged_core.rglob("*") if path.is_file())
    if [path.relative_to(staged_core) for path in staged] != [
        path.relative_to(core) for path in sources if path.is_file()
    ]:
        raise RuntimeError("Source gem core file set differs from the checkout")
    for source in sources:
        if (
            source.is_file()
            and source.read_bytes()
            != (staged_core / source.relative_to(core)).read_bytes()
        ):
            raise RuntimeError(
                f"Source gem changed core source {source.relative_to(core)}"
            )
    if (stage / "LICENSE").read_bytes() != (ROOT / "LICENSE").read_bytes():
        raise RuntimeError("Source gem changed the license")
    if (stage / "Cargo.lock").read_bytes() != (PACKAGE / "Cargo.lock").read_bytes():
        raise RuntimeError("Source gem changed the Ruby workspace lock")
    expected_workspace = (
        (PACKAGE / "Cargo.toml").read_text()
        + "\n[workspace.package]"
        + (ROOT / "Cargo.toml").read_text().split("[workspace.package]", 1)[1]
    )
    if (stage / "Cargo.toml").read_text() != expected_workspace:
        raise RuntimeError("Unexpected source gem workspace manifest")
    manifest = tomllib.loads((PACKAGE / "ext/semantic_ruby/Cargo.toml").read_text())
    staged_manifest = tomllib.loads(
        (stage / "ext/semantic_ruby/Cargo.toml").read_text()
    )
    core_path = staged_manifest["dependencies"]["semantic-core"].pop("path")
    manifest["dependencies"]["semantic-core"].pop("path")
    if (stage / "ext/semantic_ruby" / core_path).resolve() != staged_core.resolve():
        raise RuntimeError("Source gem extension does not depend on the staged core")
    if staged_manifest != manifest:
        raise RuntimeError("Unexpected extension manifest normalization")
    locked = packages(ROOT / "Cargo.lock")
    for (name, version), checksum in packages(stage / "Cargo.lock").items():
        shared = [package for package in locked if package[0] == name]
        if shared and (locked.get((name, version)) != checksum or len(shared) != 1):
            raise RuntimeError(f"Ruby lock drifted from the workspace lock for {name}")


def main():
    parser = argparse.ArgumentParser(
        description="Lint, compile, test, package, and verify the Ruby gem."
    )
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
    run([args.bundle, "exec", "standardrb"], env=environment)
    run([args.bundle, "exec", "rake", "compile", "spec"], env=environment)
    run([args.bundle, "exec", "rbs", "-I", "sig", "validate"], env=environment)
    run(
        [args.bundle, "exec", "rspec", "--tag", "~contract"],
        env={**environment, "RBS_TEST": "1"},
    )
    shutil.rmtree(PACKAGE / "pkg", ignore_errors=True)
    run([args.bundle, "exec", "rake", "gem:source"], env=environment)
    stage = PACKAGE / "pkg/source"
    check_stage(stage)
    source_gem = next((PACKAGE / "pkg").glob("genius-bench-kernel-*.gem"))
    verify(source_gem, args.ruby, args.bundle, False, source_identity(stage / CORE))
    run([args.bundle, "exec", "rake", "native", "gem"], env=environment)
    native_gem = next(
        gem
        for gem in (PACKAGE / "pkg").glob("genius-bench-kernel-*.gem")
        if gem != source_gem
    )
    verify(native_gem, args.ruby, args.bundle, True)


if __name__ == "__main__":
    main()
