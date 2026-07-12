# Legacy .wiff.scan block indexing

Status: CONFIRMED

The `.wiff.scan` file is a flat binary file that stores the actual spectrum data. It is not self-describing; instead, it relies on an external index stream located in the parent `.wiff` file's CFBF container.

## The Idx Stream

In the `.wiff` file, the stream `SampleSubtree/Sample1/Idx` (or equivalent for other samples) contains the index records that map to blocks in `.wiff.scan`.

The `Idx` stream has the following structure:
- **Header**: 32 bytes (0x20 bytes). Contains unknown metadata.
- **Records**: A contiguous array of 54-byte records.

### Index Record Layout (54 bytes)

Based on analyzing the `Idx` stream of `Rcor2KOESC1.wiff` (TripleTOF) and correlating with `.wiff.scan`:

| Offset | Type | Size | Description |
|---|---|---|---|
| 0x00 | `uint32` | 4 | Byte offset of the block in `.wiff.scan` |
| 0x04 | `uint32` | 4 | Size of the block in `.wiff.scan` in bytes |
| 0x08 | `uint32` | 4 | Unknown (typically `0x00000000`, except for the first record) |
| 0x0C | `float32` | 4 | Retention Time (in minutes) |
| 0x10 | `uint8` | 1 | MS level or scan type flag (`01` = MS1, `00` = MS2 observed) |
| 0x11 | `uint8` | 1 | Unknown (typically `0x00`) |
| 0x12 | `float64` | 8 | Total Ion Current (TIC) or Base Peak Intensity |
| 0x1A | `float64` | 8 | Constant grid spacing or baseline intensity identifier for the scan. This exactly matches the repeating baseline/gap tokens found in the compressed array. |
| 0x22 | `bytes` | 20 | Zero-padding (Trailing 20 bytes observed as all `00` in corpus tests) |

> [!NOTE]
> Unlike Waters RAW formats, the SCIEX `Idx` stream **does not** store the precursor m/z for MS2 scans. The trailing 20 bytes are universally zeroed out in the tested corpus. Precursor m/z is instead routed to the `DDERealTimeDataEx` stream for data-dependent scans or defined statically within the `MethodSubtree` (e.g. `MassRangeEx` in `Experiment` folders) for MRM/targeted scans.

## .wiff.scan blocks
The `.wiff.scan` file itself lacks a container format.
- Offset `0x00`: Global header (68 bytes long), starting with `82 05 00 00`. 
- Following this header, the file contains contiguous data blocks whose sizes vary dynamically scan-to-scan.
- Each block begins with a fixed 56-byte header containing scan-specific boundaries, followed by the variably sized compressed payload.

### Payload Decoding (Custom Token Array)
The data payload following the 56-byte header is **NOT** compressed with standard LZ/entropy coding (entropy measures ~4.2 - 4.7 bits/byte). Instead, SCIEX uses a custom byte-level prefix encoding to store an array of integer pairs `(delta_X, intensity)`. 

**Token Prefix Rules:**
The payload is parsed sequentially. The first byte of each token dictates how many following bytes to read for the value:
*   `0x00` to `0xfb` (0-251): The byte *itself* is the literal value.
*   `0xfc` (252): The next **1 byte** is the value.
*   `0xfd` (253): The next **2 bytes** are the value (little-endian).
*   `0xfe` (254): The next **3 bytes** are the value (little-endian).
*   `0xff` (255): The next **4 bytes** are the value (little-endian).

**Array Structure:**
The decoded integer stream forms an interleaved list of `(m/z_delta, intensity)` pairs.
*   In sparse background regions, `m/z_delta` is a large gap (often requiring `0xfd` prefixes) and `intensity` is the baseline noise level (e.g., 41 or 135).
*   In continuous peak regions, `m/z_delta` is the constant grid spacing for the scan (matching the `Idx` `0x1A` field), and `intensity` is the true signal peak height.
*   Because continuous gaps and baseline intensities repeat heavily, the literal byte encoding produces massive visual repetition in the raw hex (e.g. `fd 55 01 29`, where `fd 55 01` = 341 gap, `29` = 41 baseline intensity).
