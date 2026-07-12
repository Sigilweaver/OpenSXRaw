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

It currently doesn't. `Reader` reads scan data identically for both
families via the shared `Idx`/`.wiff.scan` mechanism, and reports every
spectrum with the same `Analyzer::TOFMS` / profile-mode metadata
regardless of source instrument - see
[Reader: what the reader does not yet do](./reader#what-the-reader-does-not-yet-do).
If you need to distinguish TripleTOF from QTRAP output today, check the
method-subtree contents yourself (`ExperimentTOF` vs `sMRM`/`sMRMEX`) - a
proper instrument-family detector is not yet built into the reader API.
