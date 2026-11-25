# Release Genius Bench

Each release uses the same version for the crate, Python distribution, gem,
and Go module. Update the workspace version in `Cargo.toml`, Python's
`bindings/python/pyproject.toml`, and Ruby's
`bindings/ruby/lib/genius_bench/version.rb`. Update the lockfiles and changelog
with the version change. Run `make check` and confirm the package workflows on
the candidate commit. The current Go module path supports stable v0 and v1
releases. A v2 release needs a matching major-version suffix in the module path
and consumer imports.

## Try a package before publishing

From the repository root, run the same consumer check as the Rust package job.

```sh
version=$(python3 scripts/release_version.py)
cargo package --locked --package genius-bench-kernel
bash scripts/verify_install.sh rust "${version#v}" "target/package/genius-bench-kernel-${version#v}"
```

The script creates a temporary project, installs the package, and runs the
language's grading example. Pass `python` with a wheel file or `ruby` with a
directory of candidate gems to exercise the same check through pip or Bundler.
Omit the last argument to test an exact version from its public registry.

## Registry setup

Register `shaquan1228/genius-bench` as the publisher for `genius-bench-kernel`.
Configure the corresponding GitHub environments to allow release tags.

| Destination | Workflow | Environment |
| --- | --- | --- |
| crates.io | `crate.yml` | `crates-io` |
| PyPI | `wheels.yml` | `pypi` |
| RubyGems.org | `gems.yml` | `rubygems` |
| GitHub Go module tags | `go.yml` | `go-release` |

For a new crate, provide a short-lived crates.io publish token as the
`CRATES_IO_BOOTSTRAP_TOKEN` environment secret. After the first publish,
remove it and configure crates.io Trusted Publishing for the crate. Create
pending Trusted Publishers on PyPI and RubyGems.org. Set
`RUBYGEMS_OWNER_ID` to the RubyGems.org account ID for the pending publisher.

## Publish a version

From the release commit on `main`, check the versions and push the root
release tag.

```sh
version=$(python3 scripts/release_version.py)
git tag -a "$version" -m "Genius Bench $version"
git push origin "$version"
```

The workflows build and test the packages before their publish jobs. The Go
job pushes a module tag such as
`bindings/go/genius-bench-kernel/v0.1.0` on a child of the release commit that
adds the tested Rust archives. This keeps the archives out of `main` and
lets Go fetch the complete package from the tagged tree.

Confirm the published install jobs across the four workflows. Rerun failed
jobs from the original workflow run to retain its built artifacts. For Cargo,
PyPI, and RubyGems, the publish job compares SHA-256 checksums and uploads
missing artifacts. On a checksum conflict, compare the original build artifacts
with the registry before attempting another release. The Go job checks the
existing module tag's full tree and parent against the release and its two
archives.

To adopt the Python release in Genius evals, pin its version in the lockfile and
run the no-model smoke. Record the version and source hash from the installed
core. The hash includes the core manifest, which package managers can rewrite
during packaging.
