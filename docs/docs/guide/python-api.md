---
sidebar_position: 5
---

# Python API

The `opensxraw` wheel exposes a small, eager reader built on the same Rust
core as the [Rust reader](./reader). Install it with `pip install opensxraw`
(or `pip install openmassspec[sciex]` for the umbrella).

```python
import opensxraw

reader = opensxraw.RawReader("sample.wiff")
```

`RawReader` expects the paired `.wiff.scan` file to sit alongside the
`.wiff` file (with `.scan` appended to the filename), exactly like the Rust
reader. Opening it decodes **every** spectrum up front into memory, so
construction is where the work happens and subsequent access is cheap. For
streaming access over large acquisitions, use the Rust reader's
`iter_spectra` instead.

## `RawReader`

| Member                 | Type            | Description                                                 |
| ---------------------- | --------------- | ---------------------------------------------------------- |
| `RawReader(path)`      | constructor     | Open the `.wiff`/`.wiff.scan` pair at `path` and decode it  |
| `scan_count`           | `int`           | Number of decoded spectra                                  |
| `read_spectrum(index)` | `Spectrum`      | The spectrum at zero-based `index` (raises if out of range) |

```python
print(reader.scan_count)
for i in range(reader.scan_count):
    spectrum = reader.read_spectrum(i)
    ...
```

## `Spectrum`

| Attribute             | Type          | Description                          |
| --------------------- | ------------- | ------------------------------------ |
| `mz`                  | `list[float]` | m/z values (float64)                 |
| `intensity`           | `list[float]` | Intensities (float32)                |
| `ms_level`            | `int`         | MS level (1 for MS1, 2+ for MS/MS)   |
| `retention_time_sec`  | `float`       | Retention time in seconds            |

`len(spectrum)` returns the peak count. Note the m/z values are currently
raw, uncalibrated time-bin integers, and MS2 precursor m/z is not yet
populated; see [Scan data](./scan-data) for the details and the current
[known limitations](../format/overview).

```python
spectrum = reader.read_spectrum(0)
print(spectrum.ms_level, spectrum.retention_time_sec, len(spectrum))
mz, intensity = spectrum.mz, spectrum.intensity
```

## Next

- [Reader API](./reader) (Rust)
- [Scan data layouts](./scan-data)
