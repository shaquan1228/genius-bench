import hashlib
from pathlib import Path
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]


def run(command, *, cwd=ROOT, env=None, capture=False, check=True):
    print("+ " + " ".join(map(str, command)), flush=True)
    result = subprocess.run(
        command, cwd=cwd, env=env, text=True, capture_output=capture, check=False
    )
    if check and result.returncode != 0:
        if capture:
            print(result.stdout, result.stderr, sep="\n", flush=True)
        raise subprocess.CalledProcessError(result.returncode, command)
    return result


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
