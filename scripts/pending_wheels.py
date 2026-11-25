import argparse
import hashlib
import json
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import urlopen


def pending(version, directory):
    url = f"https://pypi.org/pypi/genius-bench-kernel/{version}/json"
    try:
        with urlopen(url, timeout=30) as response:
            files = {
                entry["filename"]: entry["digests"]["sha256"]
                for entry in json.load(response)["urls"]
            }
    except HTTPError as error:
        if error.code != 404:
            raise
        files = {}

    artifacts = sorted(directory.iterdir())
    if not artifacts:
        raise ValueError("No Python release artifacts")
    for artifact in artifacts:
        if artifact.name not in files:
            continue
        with artifact.open("rb") as stream:
            checksum = hashlib.file_digest(stream, "sha256").hexdigest()
        if checksum != files[artifact.name]:
            raise ValueError(f"PyPI already contains a different {artifact.name}")
        artifact.unlink()
    return any(directory.iterdir())


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("version")
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    print("true" if pending(args.version, args.directory) else "false")
