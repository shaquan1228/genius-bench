import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "bindings/ruby"
sys.path.insert(0, str(ROOT / "bindings/python"))
from verify_wheel import source_identity  # noqa: E402


def run(command, *, cwd, env=None, capture=False):
    print("+ " + " ".join(map(str, command)), flush=True)
    result = subprocess.run(
        command, cwd=cwd, env=env, text=True, capture_output=capture, check=False
    )
    if result.returncode != 0:
        if capture:
            print(result.stdout, result.stderr, sep="\n", flush=True)
        raise subprocess.CalledProcessError(result.returncode, command)
    return result


def gem_files(gem):
    with tarfile.open(gem) as package:
        with tarfile.open(fileobj=package.extractfile("data.tar.gz")) as data:
            return data.getnames()


def ruby_query(ruby, bundle, expression):
    environment = {**os.environ, "BUNDLE_GEMFILE": str(PACKAGE / "Gemfile")}
    return run(
        [bundle, "exec", ruby, "-e", expression],
        cwd=PACKAGE,
        env=environment,
        capture=True,
    ).stdout.strip()


def cached_gems(ruby, bundle, names):
    listing = ruby_query(
        ruby,
        bundle,
        "pending = %w[" + " ".join(names) + "]; cached = {}; "
        "until pending.empty?; name = pending.shift; next if cached.key?(name); "
        "spec = Gem::Specification.find_by_name(name); cached[name] = spec.cache_file; "
        "spec.runtime_dependencies.each { |dependency| pending << dependency.name }; end; "
        'print cached.values.select { |file| File.file?(file) }.join("\\n")',
    )
    return [Path(line) for line in listing.splitlines()]


def verify(gem, ruby, bundle, native, expected_source_hash=None):
    names = gem_files(gem)
    compiled = [name for name in names if name.endswith((".bundle", ".so"))]
    sources = [name for name in names if name.endswith((".rs", "Cargo.lock"))]
    if "LICENSE" not in names:
        raise RuntimeError("Gem missing LICENSE")
    if native and (not compiled or sources):
        raise RuntimeError(f"Native gem contents are wrong: {names}")
    if not native:
        for expected in (
            "Cargo.lock",
            "Cargo.toml",
            "crates/semantic-core/build.rs",
            "crates/semantic-core/src/lib.rs",
            "ext/semantic_ruby/src/lib.rs",
            "ext/semantic_ruby/extconf.rb",
            "sig/genius_bench.rbs",
        ):
            if expected not in names:
                raise RuntimeError(f"Source gem missing {expected}")
        if compiled:
            raise RuntimeError("Source gem contains compiled artifacts")
    ruby = ruby_query(ruby, bundle, "print RbConfig.ruby")
    gem_command = str(Path(ruby).parent / "gem")
    support = cached_gems(ruby, bundle, ["rspec", "rbs"])
    build = [] if native else cached_gems(ruby, bundle, ["rb_sys"])
    with tempfile.TemporaryDirectory(prefix="kernel-gem-") as directory:
        temporary = Path(directory)
        home = temporary / "gems"
        shutil.copytree(PACKAGE / "spec", temporary / "spec")
        shutil.copy(PACKAGE / ".rspec", temporary / ".rspec")
        empty_path = temporary / "empty-path"
        empty_path.mkdir()
        clean = {
            key: value
            for key, value in os.environ.items()
            if not key.startswith(("BUNDLE", "GEM", "RUBY"))
        }
        clean["GEM_HOME"] = str(home)
        clean["GEM_PATH"] = str(home)
        install = [
            gem_command,
            "install",
            "--local",
            "--ignore-dependencies",
            "--no-document",
            "--install-dir",
            str(home),
            "--bindir",
            str(home / "bin"),
        ]
        for package in support + build:
            run([*install, str(package)], cwd=temporary, env=clean)
        compile_environment = {
            **clean,
            "RUBY": ruby,
            "CARGO_NET_OFFLINE": "true",
            "CARGO_TARGET_DIR": str(temporary / "target"),
        }
        clean["PATH"] = str(empty_path)
        run(
            [*install, str(gem)],
            cwd=temporary,
            env=clean if native else compile_environment,
        )
        clean["KERNEL_SOURCE_HASH"] = expected_source_hash or source_identity()
        clean["KERNEL_COMPILER_FREE"] = "1"
        run([ruby, "spec/smoke.rb"], cwd=temporary, env=clean)
        run([ruby, str(home / "bin/rspec"), "spec"], cwd=temporary, env=clean)
        print(
            f"Installed {'native' if native else 'source'} gem: boundary, identity, and signature checks passed.",
            flush=True,
        )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--gem", required=True, type=Path)
    parser.add_argument("--ruby", default=os.environ.get("RUBY", "ruby"))
    parser.add_argument("--bundle", default=os.environ.get("BUNDLE", "bundle"))
    parser.add_argument("--native", action="store_true")
    args = parser.parse_args()
    verify(args.gem.resolve(), args.ruby, args.bundle, args.native)


if __name__ == "__main__":
    main()
