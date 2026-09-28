"""Real-file smoke test for the SCIEX Python field surface."""

from __future__ import annotations

import os
from pathlib import Path

import opensxraw
import pytest


def test_decoded_fields():
    path = Path(os.environ.get("OPENSXRAW_TEST_WIFF", "corpus/PXD022088/Rcor2KOESC1.wiff"))
    if not path.is_file():
        pytest.skip("set OPENSXRAW_TEST_WIFF to a WIFF file")
    samples = opensxraw.list_samples(str(path))
    assert samples
    reader = opensxraw.open_sample(str(path), samples[0])
    assert reader.sample_name == samples[0]
    run = reader.run_info()
    assert run["source_file_name"] == path.name
    assert run["extra"]["opensxraw.sample"] == samples[0]
    assert reader.instrument_info() is None or reader.instrument_info()["component_id"]
    assert reader.calibration() is None or isinstance(reader.calibration()["slope"], float)
    assert isinstance(reader.ion_source_parameters(), dict)
    assert isinstance(reader.dde_precursor_mz(), list)
    assert len(reader.scan_index()) == reader.scan_count
    first = reader.read_spectrum(0)
    streamed = next(opensxraw.iter_spectra(str(path), sample=samples[0]))
    assert first.native_id == streamed.native_id
    assert len(first.mz) == len(first.intensity)
    for chrom in reader.read_chromatograms():
        assert len(chrom["time_sec"]) == len(chrom["intensity"])
