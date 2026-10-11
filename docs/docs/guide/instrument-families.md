---
sidebar_position: 2
---

# Instrument families

OpenSXRaw's corpus covers two SCIEX instrument families that both use the
legacy `.wiff`/`.wiff.scan` container:

## TripleTOF

Quadrupole time-of-flight instruments (TripleTOF 5600/6600 confirmed in
the validation corpus). TripleTOF acquisitions run high-resolution
DDA (data-dependent acquisition) or DIA/SWATH workflows, and make heavy
use of TOF-specific calibration telemetry (`TOFCalibrationData`,
`TDCStatistics`) and `ExperimentTOF` method blocks - see
[Format specification](../format/legacy-wiff-cfbf).

## QTRAP

Triple-quadrupole linear ion trap instruments (QTRAP 5500/6500 confirmed
in the validation corpus) typically running scheduled MRM/SRM targeted
acquisitions (`sMRM`/`sMRMEX` method blocks). QTRAP is nominal-mass, not
true time-of-flight, unlike TripleTOF.

## How the reader tells them apart

The reader uses the `TOFCalibrationData` stream. TripleTOF and ZenoTOF
files carry a `(slope, intercept)` body there; QTRAP files carry only a
header. When the body is present, spectra are reported with
`Analyzer::TOFMS` and m/z from a per-file linear calibration (not
accurate, see Known issues in the README); otherwise with `Analyzer::TQMS`
and raw-bin m/z. Across the 200-file local corpus this signal agreed with the
presence of `ExperimentTOF` method streams and with every embedded
instrument-name hint. When the `Log` stream carries a self-identification
record, `run_metadata().instrument` also names the specific model.
