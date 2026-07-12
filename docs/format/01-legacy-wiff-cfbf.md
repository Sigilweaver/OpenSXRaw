# Legacy .wiff CFBF Container Structure

Status: DECODED (COMPLETED)

Legacy `.wiff` files utilize the Microsoft Compound File Binary Format (CFBF/OLE2) to store hierarchical metadata, method configuration, sample details, and indexing for the corresponding `.wiff.scan` file.

Below is a comprehensive catalog of the stream tree structures found across both QTRAP and TripleTOF corpus files. 

## General CFBF Structure

The `.wiff` container is structured into two main subtrees:
1. `MethodSubtree`: Contains the instrumental acquisition methods, MS parameters, and audit trails.
2. `SampleSubtree`: Contains sample tracking, hardware device logs, and the crucial `Idx` stream that maps to the `.wiff.scan` payload.

### Method Subtree (`MethodSubtree/`)
This storage acts as a structured directory of instrument methods.

*   `Method1/MethodHeader`: General metadata for the acquisition method.
*   `Method1/GLPTables`: Stores "Good Laboratory Practice" audit records. This is a crucial historical precedent indicating SCIEX's requirement for strict, untamperable audit trails.
*   `Method1/DeviceMethodX/`: Configurations for specific hardware components (e.g., LC pumps, autosamplers, MS).
    *   `ADCMethod`, `ADCChannelX`: Analog-to-digital converter configurations.
    *   `VendorAppMethod`: Opaque blob storing specific third-party LC/autosampler method settings.
    *   `PeriodX/ExperimentY/`: The core MS method details for a given experiment (e.g., MRM transition lists, SWATH windows).
        *   `ExperimentHeader`, `ExperimentHeaderEx`
        *   `IonSourceParamsTable/`: Contains `ParamCollHeader` and sub-directories (`Parameter0`, `Parameter1`, etc.) holding source voltage/gas settings (`ParameterData`).
        *   `MassRangeEx/`
        *   `sMRM`, `sMRMEX`: Specific to targeted MRM acquisitions (prominent in QTRAP).
        *   `ExperimentTOF`: Specific to Time-of-Flight acquisitions (prominent in TripleTOF).

### Sample Subtree (`SampleSubtree/`)
This storage tracks the execution of a specific sample acquisition.

*   `SampleTable`, `SampleIdxTable`, `DabsInfo`: High-level sample list and indexing.
*   `Sample1/Log`: Contains acquisition system logs.
*   `Sample1/RealTimeSettings`: Run-time parameters applied during acquisition.
*   `Sample1/Idx`: **Crucial Index Stream**. Contains byte offsets into the `.wiff.scan` file mapping scan indices to raw spectrum payloads.
*   `Sample1/DDERealTimeData`, `DDERealTimeDataEx`: Contains real-time MS metadata (e.g., precursor m/z for DDA scans).
*   `Sample1/Devices/`: Contains run-time telemetry and channel data for external devices (`Device_0`, `Device_1`, etc.).
    *   `DevRealTimeHeader`, `DevData`, `Channel`.
*   `Sample1/PeakFinder/PeakFinderInfo`: Settings or state for embedded peak detection.
*   `Sample1/SampleDABE/`: Sample data and compliance information.
    *   `CFR_INFO`: Stores "21 CFR Part 11" compliance information (electronic signatures and record protection). This, alongside `GLPTables`, heavily implies SCIEX's engineering requirement to prevent external modification of method/metadata.
    *   `DATA`, `PEAK_NUM`.
*   `Sample1/TDCStatistics`, `Sample1/TOFCalibrationData`: High-volume calibration and time-to-digital converter telemetry (Prominent in TripleTOF datasets).

## Instrument Variations

**TripleTOF:**
- Heavy reliance on TOF-specific calibration telemetry streams (`TOFCalibrationData`, `TDCStatistics`).
- Uses `ExperimentTOF` blocks in the method subtree.
- Often includes Information Dependent Acquisition (`IDA`) configuration blocks.

**QTRAP:**
- Heavy reliance on scheduled MRM configurations (`sMRM`, `sMRMEX`).
- Contains specialized QTRAP experiment headers (`ExperimentHeaderQTrapEX`).

## Conclusion on Security and Obfuscation

The legacy `.wiff` format relies heavily on compliance auditing (`CFR_INFO` for 21 CFR Part 11, `GLPTables` for Good Laboratory Practice). While the CFBF container itself is public and unencrypted, the binary contents of the methods and logs are encoded in ways that make manipulation difficult.

This deep integration of regulatory compliance (anti-tampering, auditability) is the direct historical precedent for the `.wiff2` format. When moving to `.wiff2`, SCIEX consolidated this entire structure into a unified, SQLCipher-encrypted database, natively fulfilling the 21 CFR Part 11 requirements by completely denying external read/write access to the metadata without the vendor DLL (`Clearcore2.Data`).
