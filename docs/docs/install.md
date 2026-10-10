---
sidebar_position: 2
---

# Install

OpenSXRaw is published to crates.io and PyPI.

## Rust

```sh
cargo add opensxraw
```

## Python

```sh
pip install opensxraw
```

OpenSXRaw needs Rust 1.88 or newer. There are no native or system
dependencies.

## Verifying the install

```sh
cargo test --workspace
```

## Optional: corpus fetcher

The validation corpus is not redistributed. It is pulled on demand from
the [PRIDE Archive](https://www.ebi.ac.uk/pride/) using local research
tooling (not part of the published crate):

```sh
python -m analysis.pride fetch <PXD_ACCESSION>
```

See [`CORPUS.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CORPUS.md)
for the file list and provenance.

The conformance tests look for fixtures under `$OPENSXRAW_CORPUS/<PXD>/`
(for example `$OPENSXRAW_CORPUS/PXD022088/Rcor2KOESC1.wiff`) and then under
the repo-root `corpus/` directory. Missing fixtures are skipped unless
`REQUIRE_CORPUS=1` is set, in which case they fail:

```sh
OPENSXRAW_CORPUS=/path/to/SRaw REQUIRE_CORPUS=1 cargo test --workspace
```
