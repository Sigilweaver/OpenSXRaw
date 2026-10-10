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
| `.wiff2` container                            | not supported (encrypted, out of scope) - see [Format specification](./format/wiff2-container) |
| m/z calibration (`TOFCalibrationData`)        | TripleTOF/ZenoTOF: calibrated; QTRAP: raw time-bin values |
| MS2 precursor m/z (`DDERealTimeDataEx`)        | decoded on DDA files (cycle-linked heuristic) |

CI runs the conformance suite against a public QTRAP 6500+ fixture.
TripleTOF files currently decode with every scan labelled MS2, including
survey scans; see Known issues in the README.

## Next steps

- [Install](./install) the Rust crate.
- Run through the [Quickstart](./quickstart).
- Read the [Format specification](./format/overview) for the binary
  layer.

## License

OpenSXRaw is Apache-2.0 licensed. See [License](./license).
