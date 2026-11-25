# Release Genius Bench

Each release uses the same version for the crate, Python distribution, gem,
and Go module. Update the workspace version in `Cargo.toml`, Python's
`bindings/python/pyproject.toml`, and Ruby's
`bindings/ruby/lib/genius_bench/version.rb`. Update the lockfiles and changelog
with the version change. Run `make check` and confirm the package workflows on
the candidate commit.

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
jobs from the original workflow run to retain its built artifacts. To adopt the
Python release in Genius evals, use an exact version in its lockfile and run the
no-model smoke. Preserve grading build identity in the resulting evidence.
