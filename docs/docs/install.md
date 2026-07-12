---
sidebar_position: 2
---

# Install

OpenSXRaw is not yet published to crates.io. Until then, use it from
source.

## Rust

```toml
[dependencies]
opensxraw = { git = "https://github.com/Sigilweaver/OpenSXRaw" }
```

OpenSXRaw needs Rust 1.85 or newer. There are no native or system
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
