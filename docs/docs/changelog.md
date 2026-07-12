---
sidebar_position: 98
---

# Changelog

The canonical changelog lives at
[`CHANGELOG.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CHANGELOG.md)
in the repository root. The notes below mirror the latest state.

## 0.1.0 - 2026-07-11

First release. Published on crates.io (`opensxraw`).

- Rust reader (`opensxraw`) for legacy SCIEX `.wiff`/`.wiff.scan` files,
  covering TripleTOF and QTRAP instrument families.
- Full CFBF stream catalog and `.wiff.scan` block/token-stream decoding.
- `.wiff2` container investigation: confirmed proprietary AES page
  encryption and structural analysis of the plaintext/ciphertext
  boundary - see [Format specification](./format/wiff2-container).
  Support remains deferred pending new information.
