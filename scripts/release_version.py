import argparse
from pathlib import Path
import re
import tomllib


ROOT = Path(__file__).resolve().parents[1]


def manifest(path):
    with (ROOT / path).open("rb") as source:
        return tomllib.load(source)


def release_tag(rust, python, ruby):
    if not rust == python == ruby:
        raise ValueError(
            f"Release versions disagree: Rust={rust}, Python={python}, Ruby={ruby}"
        )
    if not re.fullmatch(r"(?:0|[1-9][0-9]*)(?:\.(?:0|[1-9][0-9]*)){2}", rust):
        raise ValueError(f"Expected a stable major.minor.patch version, got {rust!r}")
    if rust.split(".")[0] not in {"0", "1"}:
        raise ValueError("Releasing v2 or later requires a versioned Go module path")
    return f"v{rust}"


def main():
    parser = argparse.ArgumentParser(
        description="Check the release versions and print the root Git tag."
    )
    parser.add_argument("tag", nargs="?", help="expected root Git tag")
    args = parser.parse_args()
    ruby_source = (ROOT / "bindings/ruby/lib/genius_bench/version.rb").read_text()
    ruby = re.search(r'^\s*VERSION = "([^"]+)"$', ruby_source, flags=re.MULTILINE)
    if ruby is None:
        parser.exit(1, "Cannot find Ruby package version\n")
    try:
        tag = release_tag(
            manifest("Cargo.toml")["workspace"]["package"]["version"],
            manifest("bindings/python/pyproject.toml")["project"]["version"],
            ruby[1],
        )
    except ValueError as error:
        parser.exit(1, f"{error}\n")
    if args.tag is not None and args.tag != tag:
        parser.exit(1, f"Expected release tag {tag}, got {args.tag}\n")
    print(tag)


if __name__ == "__main__":
    main()
