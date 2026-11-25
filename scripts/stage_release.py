import argparse
import hashlib
import json
import shutil
from fnmatch import fnmatchcase
from pathlib import Path
from tempfile import TemporaryDirectory
from urllib.error import HTTPError
from urllib.request import Request, urlopen

PACKAGE = "genius-bench-kernel"
PYTHON_PACKAGE = PACKAGE.replace("-", "_")


def published_checksums(registry, version):
    urls = {
        "crates-io": f"https://crates.io/api/v1/crates/{PACKAGE}/{version}",
        "pypi": f"https://pypi.org/pypi/{PACKAGE}/{version}/json",
        "rubygems": f"https://rubygems.org/api/v1/versions/{PACKAGE}.json",
    }
    request = Request(
        urls[registry],
        headers={
            "User-Agent": "genius-bench-release (github.com/shaquan1228/genius-bench)"
        },
    )
    try:
        with urlopen(request, timeout=30) as response:
            data = json.load(response)
    except HTTPError as error:
        error.close()
        if error.code == 404:
            return {}
        raise

    if registry == "crates-io":
        release = data["version"]
        if release["num"] != version:
            raise ValueError(f"Unexpected crates.io version: {release['num']}")
        return {
            f"{PACKAGE}-{version}.crate": None
            if release["yanked"]
            else release["checksum"]
        }
    if registry == "pypi":
        return {
            entry["filename"]: None if entry["yanked"] else entry["digests"]["sha256"]
            for entry in data["urls"]
        }

    gems = {}
    for entry in data:
        if entry["number"] == version:
            platform = entry["platform"]
            suffix = "" if platform == "ruby" else f"-{platform}"
            gems[f"{PACKAGE}-{version}{suffix}.gem"] = entry["sha"]
    return gems


def stage_release(registry, version, artifacts, pending):
    patterns = {
        "crates-io": [f"{PACKAGE}-{version}.crate"],
        "pypi": [
            f"{PYTHON_PACKAGE}-{version}-*.whl",
            f"{PYTHON_PACKAGE}-{version}.tar.gz",
        ],
        "rubygems": [f"{PACKAGE}-{version}.gem", f"{PACKAGE}-{version}-*.gem"],
    }
    files = sorted(artifacts.iterdir())
    if not files:
        raise ValueError(f"No artifacts in {artifacts}")
    if pending.exists():
        raise ValueError(f"Staging directory already exists: {pending}")
    for artifact in files:
        if not artifact.is_file() or not any(
            fnmatchcase(artifact.name, pattern) for pattern in patterns[registry]
        ):
            raise ValueError(
                f"Unexpected {registry} artifact for {version}: {artifact}"
            )

    published = published_checksums(registry, version)
    missing = []
    for artifact in files:
        if artifact.name not in published:
            missing.append(artifact)
            continue
        checksum = published[artifact.name]
        if checksum is None:
            raise ValueError(f"{registry} has yanked {artifact.name}")
        with artifact.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != checksum:
                raise ValueError(
                    f"{registry} already contains a different {artifact.name}"
                )

    with TemporaryDirectory(dir=pending.parent) as workspace:
        staged = Path(workspace) / "pending"
        staged.mkdir()
        for artifact in missing:
            shutil.copyfile(artifact, staged / artifact.name)
        staged.rename(pending)
    return bool(missing)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Stage missing release files after verifying published checksums."
    )
    parser.add_argument("registry", choices=["crates-io", "pypi", "rubygems"])
    parser.add_argument("version")
    parser.add_argument("artifacts", type=Path)
    parser.add_argument("pending", type=Path)
    args = parser.parse_args()
    print(
        "true"
        if stage_release(args.registry, args.version, args.artifacts, args.pending)
        else "false"
    )
