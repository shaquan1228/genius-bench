from pathlib import Path
import re
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]


def manifest(path):
    return tomllib.loads((ROOT / path).read_text())


rust = manifest("Cargo.toml")["workspace"]["package"]["version"]
python = manifest("bindings/python/pyproject.toml")["project"]["version"]
ruby_file = ROOT / "bindings/ruby/lib/genius_bench/version.rb"
ruby = re.search(r'VERSION = "([^"]+)"', ruby_file.read_text())
if (
    not ruby
    or not rust == python == ruby[1]
    or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", rust)
):
    raise SystemExit(
        "Rust, Python, and Ruby must declare the same stable release version"
    )
tag = f"v{rust}"
if len(sys.argv) > 1 and sys.argv[1] != tag:
    raise SystemExit(f"Expected release tag {tag}, got {sys.argv[1]}")
print(tag)
