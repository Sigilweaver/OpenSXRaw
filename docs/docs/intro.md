---
sidebar_position: 1
slug: /
---

# OpenSXRaw

:::info Part of the OpenMassSpec stack

OpenSXRaw is one of the vendor readers in
[OpenMassSpec](https://sigilweaver.app/openmassspec/docs/), a Rust- and
Python-native stack for proteomics raw-file access. Sibling readers:
[OpenTFRaw](https://sigilweaver.app/opentfraw/docs/) (Thermo `.raw`),
[OpenWRaw](https://sigilweaver.app/openwraw/docs/) (Waters `.raw/`),
[OpenTimsTDF](https://sigilweaver.app/opentimstdf/docs/) (Bruker `.d/`),
[OpenARaw](https://sigilweaver.app/openaraw/docs/) (Agilent `.d/`).

:::

OpenSXRaw is a Rust library that reads SCIEX `.wiff`/`.wiff.scan` legacy
mass-spectrometry data files - the paired-file binary format produced by
SCIEX TripleTOF and QTRAP instruments running Analyst acquisition
software.

It runs on Linux, macOS, and Windows, with no dependency on any SCIEX SDK
or software. The format was decoded by clean-room binary analysis of a
public corpus of mass-spectrometry datasets (PRIDE accessions); see
[`CORPUS.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CORPUS.md).

## What it covers

| Component                                    | Status    |
| --------------------------------------------- | --------- |
| `.wiff` CFBF container (method/sample metadata, `Idx` stream) | supported |
| `.wiff.scan` block index and token-stream decoding | supported |
| TripleTOF instrument family                   | supported |
| QTRAP instrument family                       | supported |
| `.wiff2` container                            | investigated, not readable - see [Format specification](./format/wiff2-container) |
| m/z calibration (`ExperimentTOF` constants)    | not yet decoded - m/z is a raw time-bin integer |
| MS2 precursor m/z (`DDERealTimeDataEx`)        | not yet decoded |

Validated against real-world TripleTOF 5600 corpus data end to end (2228
scans decoded from a single fixture run); broader corpus-wide conformance
testing has not been run yet.

## Next steps

- [Install](./install) the Rust crate.
- Run through the [Quickstart](./quickstart).
- Read the [Format specification](./format/overview) for the binary
  layer.

## License

OpenSXRaw is Apache-2.0 licensed. See [License](./license).
