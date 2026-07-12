---
sidebar_position: 1
---

# Overview

SCIEX legacy acquisitions are a pair of files: `sample.wiff` (metadata,
methods, and the scan index) and `sample.wiff.scan` (the actual spectrum
payload). Neither file is self-describing on its own - `.wiff.scan` has
no container format at all, and relies entirely on an index stream inside
the paired `.wiff` file to locate its data blocks.

```
sample.wiff         - CFBF/OLE2 container: methods, sample metadata, Idx stream
sample.wiff.scan    - flat binary file: global header + variable-length scan blocks
```

`.wiff` uses Microsoft's public Compound File Binary Format (CFBF/OLE2) -
reading its stream tree is not reverse engineering SCIEX's own work, only
the contents and layout of the streams inside it are.

## Files

| File | Purpose | Status |
| --- | --- | --- |
| [Legacy .wiff CFBF container](./legacy-wiff-cfbf) | Method/sample metadata stream tree, including the `Idx` scan index | Confirmed |
| [Legacy .wiff.scan blocks](./legacy-wiff-scan) | Block index layout and the custom token-stream payload encoding | Confirmed |
| [.wiff2 container](./wiff2-container) | SCIEX's newer, self-contained, proprietary-encrypted format | Unsolved, not readable |

## Clean-room provenance

Every byte-level claim on these pages came from binary analysis of the
public PRIDE corpus (see
[`CORPUS.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CORPUS.md))
plus the public CFBF/OLE2 container specification. No SCIEX SDK, Analyst
software, or other vendor tooling was used at any point - see
[`CONTRIBUTING.md`](https://github.com/Sigilweaver/OpenSXRaw/blob/main/CONTRIBUTING.md#vendor-software-and-clean-room-policy).
