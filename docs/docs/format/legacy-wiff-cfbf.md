---
sidebar_position: 2
---

# Legacy .wiff CFBF container

## Status: Confirmed

`.wiff` files use the Microsoft Compound File Binary Format (CFBF/OLE2)
to store hierarchical metadata, method configuration, sample details, and
the scan index for the paired `.wiff.scan` file. The container is
structured into two main subtrees.

## Method subtree (`MethodSubtree/`)

Structured directory of instrument methods:

- `Method1/MethodHeader` - general acquisition method metadata.
- `Method1/GLPTables` - Good Laboratory Practice audit records.
- `Method1/DeviceMethodX/` - per-hardware-component configuration (LC
  pumps, autosamplers, MS), including `PeriodX/ExperimentY/` blocks with
  the core MS method details (MRM transition lists, SWATH windows,
  source voltage/gas settings). `ExperimentTOF` blocks are TripleTOF-
  specific; `sMRM`/`sMRMEX` blocks are QTRAP-specific.

## Sample subtree (`SampleSubtree/`)

Tracks the execution of a specific sample acquisition:

- `SampleTable`, `SampleIdxTable`, `DabsInfo` - high-level sample list
  and indexing.
- `Sample1/Idx` - **the scan index**. Maps scan indices to byte ranges in
  the paired `.wiff.scan` file. See
  [Legacy .wiff.scan blocks](./legacy-wiff-scan) for its record layout.
- `Sample1/DDERealTimeData`, `DDERealTimeDataEx` - real-time MS metadata,
  including precursor m/z for data-dependent MS2 scans. Not yet decoded
  by this reader - see [Reader](../guide/reader#what-the-reader-does-not-yet-do).
- `Sample1/SampleDABE/CFR_INFO` - 21 CFR Part 11 compliance data
  (electronic signatures, record protection).
- `Sample1/TDCStatistics`, `TOFCalibrationData` - TOF calibration
  telemetry, prominent in TripleTOF files.

## Instrument variations

**TripleTOF**: heavy use of TOF-specific calibration telemetry
(`TOFCalibrationData`, `TDCStatistics`) and `ExperimentTOF` method
blocks; often includes Information Dependent Acquisition (IDA)
configuration.

**QTRAP**: heavy use of scheduled MRM configuration (`sMRM`, `sMRMEX`)
and QTRAP-specific experiment headers.

## Relationship to .wiff2

The compliance-auditing streams here (`CFR_INFO` for 21 CFR Part 11,
`GLPTables` for GLP) are the direct historical precedent for `.wiff2`:
SCIEX later consolidated this entire structure into a single
SQLCipher-encrypted database, natively satisfying the same compliance
requirements by denying external read/write access to the metadata
without the vendor SDK. See [.wiff2 container](./wiff2-container).
