# Contributors

Thank you to everyone who has contributed to OpenSXRaw.

## Benjamin Riley ([@Nabejo](https://github.com/Nabejo))

Contributed in v0.2.2:

- **Bounded `read_scan_block` allocation** - fixed a memory-DoS where a
  crafted or corrupted Idx offset could force a multi-gigabyte read
  buffer allocation; the read length is now bounded by the Idx's own
  `scan_size`, the actual `.wiff.scan` file size, and an absolute
  ceiling.
- **Decoder unit tests** - synthetic byte-slice tests for `IdxRecord`
  parsing, the `scan.rs` terminator scan, `read_scan_block`'s offset
  bounds, and `points_to_arrays`, none requiring the out-of-tree corpus.
