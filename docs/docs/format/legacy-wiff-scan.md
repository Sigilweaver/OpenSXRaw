---
sidebar_position: 3
---

# Legacy .wiff.scan blocks

## Status: Confirmed

`.wiff.scan` is a flat binary file with no container format of its own -
it relies entirely on the `Idx` stream in the paired `.wiff` file (see
[Legacy .wiff CFBF container](./legacy-wiff-cfbf)) to locate its data
blocks.

## The Idx stream

`SampleSubtree/Sample1/Idx` holds a 32-byte header followed by a
contiguous array of 54-byte records, one per scan:

| Offset | Type | Description |
|---|---|---|
| 0x00 | u32 | Byte offset of the block in `.wiff.scan` |
| 0x04 | u32 | Byte size of the block |
| 0x0C | f32 | Retention time (minutes) |
| 0x10 | u8 | MS level flag (`1` = MS1, `0` = MS2) |
| 0x12 | f64 | Total ion current (cps) |
| 0x1A | f64 | Grid-spacing-related field (not fully resolved) |

Unlike some vendor formats, the `Idx` stream does **not** store precursor
m/z for MS2 scans - that lives in the not-yet-decoded
`DDERealTimeDataEx` stream (data-dependent scans) or is defined
statically in the method subtree (MRM/targeted scans).

## Scan blocks

Each `.wiff.scan` block starts with a fixed 56-byte header (scan-specific
boundaries), followed by a variable-length compressed payload.

## Payload encoding

The payload is **not** standard LZ/entropy compression (measured entropy
~4.2-4.7 bits/byte) - it's a custom byte-level prefix encoding for an
array of `(delta_mz, intensity)` integer pairs:

| Prefix byte | Meaning |
|---|---|
| `0x00`-`0xfb` | Literal value (the byte itself) |
| `0xfc` | Next 1 byte is the value |
| `0xfd` | Next 2 bytes (little-endian) |
| `0xfe` | Next 3 bytes (little-endian) |
| `0xff` | Next 4 bytes (little-endian) |

In sparse background regions, `delta_mz` is a large gap and `intensity`
is baseline noise; in continuous peak regions, `delta_mz` is the scan's
constant grid spacing and `intensity` is the true signal height. The
reader currently exposes `mz` as the raw accumulated time-bin integer
(not a calibrated Da value) and drops zero-intensity points as background
artifacts - see
[Reader](../guide/reader#what-the-reader-does-not-yet-do).
