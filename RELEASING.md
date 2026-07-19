# Releasing OpenSXRaw

This repo publishes one crate (`opensxraw`, crates.io) and one Python
package (`opensxraw`, PyPI, built from `crates/opensxraw-py`). Version is
a single source of truth: `workspace.package.version` in the root
`Cargo.toml`. `opensxraw-py` inherits it via `version.workspace = true`
and is `publish = false` in Cargo (it never goes to crates.io, only
PyPI). `pyproject.toml` reads `dynamic = ["version"]`, so maturin pulls
the version from Cargo.toml at build time - there is nothing to bump in
`pyproject.toml`.

## Steps

1. **Bump the version.** Edit `workspace.package.version` in
   [`Cargo.toml`](Cargo.toml) to the new version.

2. **Update the changelog.** Move the `## [Unreleased]` entries in
   [`CHANGELOG.md`](CHANGELOG.md) under a new `## [X.Y.Z] - YYYY-MM-DD`
   heading (see prior entries for the format; this repo follows [Keep a
   Changelog](https://keepachangelog.com/en/1.1.0/) and
   [SemVer](https://semver.org/)).

3. **Commit and push to `main`.** Use a `release: vX.Y.Z` commit message
   (see e.g. `5fbee87`). Push directly to `main` - this is a
   single-maintainer repo and releases don't go through a PR. Pushing
   triggers both `ci.yml` (runs on every push to `main`) and, since the
   commit touches `Cargo.toml`, `audit.yml` (path-filtered on
   `**/Cargo.toml` and `Cargo.lock`).

4. **Confirm the release commit is actually green before tagging.**
   `publish.yml` triggers straight off the `v*` tag push with no
   dependency on CI or audit passing - GitHub Actions can't make one
   workflow file `needs:` a job in another workflow file, so this has to
   be checked by hand before the tag exists, not enforced by the publish
   workflow itself. Run:

   ```
   ./scripts/check-release-ready.sh
   ```

   (defaults to `HEAD`; pass a ref/SHA to check something else). It
   queries the most recent `ci.yml` and `audit.yml` runs for the commit
   via `gh run list` and fails if either hasn't run, is still in
   progress, or didn't succeed. Do not tag until it exits 0.

   Note: `audit.yml` only runs on pushes/PRs that touch `Cargo.toml`/
   `Cargo.lock`, plus a weekly schedule - it will not have a run for the
   exact SHA of an arbitrary commit that didn't touch dependencies. Since
   the release commit itself bumps `Cargo.toml`, pushing it (step 3)
   is what gives that commit a fresh audit run to check in this step.

5. **Tag and push the tag.**

   ```
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin vX.Y.Z
   ```

   The tag push triggers `publish.yml`: `cargo publish -p opensxraw` to
   crates.io, plus wheel/sdist builds and a PyPI publish (via trusted
   publishing - no token needed) for `opensxraw`.

6. **Verify.** Check the `publish.yml` run
   (https://github.com/Sigilweaver/OpenSXRaw/actions/workflows/publish.yml)
   completed successfully, then confirm the new version shows up on
   [crates.io](https://crates.io/crates/opensxraw) and
   [PyPI](https://pypi.org/project/opensxraw/).
