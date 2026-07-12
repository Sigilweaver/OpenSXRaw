---
sidebar_position: 4
---

# .wiff2 container

## Status: Concluded, unsolved (2026-07-08)

`.wiff2` is SCIEX's newer format that combines the legacy `.wiff` metadata
and `.wiff.scan` data into a single, self-contained file. It is not
readable by OpenSXRaw. This page summarizes the investigation and its
conclusion; the full technical record (byte-level evidence, every
hypothesis tried, and why) lives in
[`docs/format/03-wiff2-container.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/docs/format/03-wiff2-container.md)
in the repository - this page is a curated summary, not a replacement for
it.

## Sources

Everything in this investigation traces to public information: the corpus
itself (files downloaded from the public EBI PRIDE Archive under open
licenses), ProteoWizard's own public open-source code and build scripts
(consulted as a reference the same way any public project would be - never
used or run to actually read `.wiff2` data, since it links SCIEX's
proprietary libraries to do so), public GitHub code search, public
cryptography references (SQLite/SQLCipher specs), and SCIEX's own public
knowledge-base articles. No SCIEX SDK, software, or vendor binary was ever
used, disassembled, or decompiled. See the full technical record linked
above for the complete source list.

## What's confirmed

Established by direct byte-level comparison across 51 `.wiff2` files
spanning 3 independent PRIDE projects/institutions/instruments:

- The file is SQLite-page-structured (4096-byte pages), with a genuine
  24-byte **plaintext header** at the start of page 0 - bytes 16-23 are
  byte-for-byte identical across all 51 files and decode exactly to real
  SQLite page-1 header fields (page size, format version, reserved-bytes
  count, and three spec-mandated constants). This rules out those bytes
  being ciphertext.
- Everything from byte 24 onward, on every page, is high-entropy
  (~7.95-7.96 bits/byte) and statistically indistinguishable from strong
  cipher output - consistent with AES-class encryption, not compression
  (compression was directly tested and ruled out).
- The encryption does not match any of the seven well-known open-source
  SQLite encryption schemes checked (SQLCipher, wxSQLite3, sqleet, Ascon,
  AEGIS, etc.) - none produce the observed 12-byte page reserve. This is
  consistent with SCIEX's own documentation describing `.wiff2` as having
  "advanced data integrity mechanisms" - i.e. a bespoke implementation,
  not a stock library.
- The key is very likely a single fixed value compiled into SCIEX's SDK
  (not a per-install or server-issued key, since third-party tools that
  link the SDK work fully air-gapped) - and plausibly GUID-shaped rather
  than a human-chosen passphrase, given the amount of custom engineering
  evident elsewhere in the format.

## Open questions

- **Cipher mode**: plain AES-CBC, AES-CBC with ciphertext stealing, or an
  AEAD mode (GCM-style) with a derived nonce - not resolvable from
  ciphertext alone.
- **KDF parameters**: iteration count and hash algorithm were never
  empirically confirmed; the campaign's assumption (historical SQLCipher
  v3-era defaults) is plausible but unconfirmed, especially given the
  encryption is confirmed non-standard in at least one other respect
  (the page reserve size).
- **Shared-key assumption**: whether every `.wiff2` file really does
  share one fixed key was argued circumstantially, not directly
  confirmed.

This can't be resolved from ciphertext alone, and won't be without either
a lucky external lead (a leaked key, a public SDK artifact) or a level of
tooling investment not currently planned. Legacy `.wiff`/`.wiff.scan`
support is unaffected either way.
