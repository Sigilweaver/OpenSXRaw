"""Real-file smoke test for the SCIEX Python field surface."""

from __future__ import annotations

import os
from pathlib import Path

import opensxraw
import pytest


FIXTURE = "PXD022088/Rcor2KOESC1.wiff"


def _fixture_path() -> Path:
    if "OPENSXRAW_TEST_WIFF" in os.environ:
        return Path(os.environ["OPENSXRAW_TEST_WIFF"])
    root = os.environ.get("OPENSXRAW_CORPUS", "corpus")
    return Path(root) / FIXTURE


def test_decoded_fields():
    path = _fixture_path()
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but {path} is missing")
        pytest.skip("set OPENSXRAW_CORPUS or OPENSXRAW_TEST_WIFF to a WIFF fixture")
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
    assert first.native_id == "scan=1"
    assert run["native_id_format"]["accession"] == "MS:1000776"
    assert len(first.mz) == len(first.intensity)
    with pytest.raises(IndexError):
        reader.read_spectrum(reader.scan_count)
    for chrom in reader.read_chromatograms():
        assert len(chrom["time_sec"]) == len(chrom["intensity"])


def test_record_fields_follow_core_4():
    path = _fixture_path()
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but {path} is missing")
        pytest.skip("set OPENSXRAW_CORPUS or OPENSXRAW_TEST_WIFF to a WIFF fixture")
    reader = opensxraw.RawReader(str(path))
    spectra = [reader.read_spectrum(i) for i in range(reader.scan_count)]
    ms2 = next(s for s in spectra if s.ms_level >= 2 and s.precursor is not None)
    precursor = ms2.precursor
    # Collision energy is a list of {"value", "unit"} dicts. The file carries
    # none, so the list is empty and the old scalar keys are gone.
    assert precursor["collision_energies"] == []
    assert "collision_energy" not in precursor
    assert "ce_is_nce" not in precursor
    assert set(precursor) == {
        "target_mz",
        "selected_mz",
        "isolation_width",
        "charge",
        "intensity",
        "collision_energies",
        "precursor_native_id",
        "activation",
        "analyzer",
        "ccs",
    }
    assert precursor["activation"] is None
    # Labels come from Core's `as_str`.
    assert ms2.analyzer == "tqms"
    assert ms2.polarity == "positive"
    # The file has no centroid/profile indicator, so the mode stays unknown.
    assert ms2.scan_mode is None
    assert isinstance(ms2.retention_time_sec, float)


def test_spectrum_warnings_field():
    """`warnings` lists what the reader flagged on a spectrum, as code and message."""
    path = _fixture_path()
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but {path} is missing")
        pytest.skip("set OPENSXRAW_CORPUS or OPENSXRAW_TEST_WIFF to a WIFF fixture")
    reader = opensxraw.RawReader(str(path))
    for i in range(reader.scan_count):
        spectrum = reader.read_spectrum(i)
        warnings = spectrum.warnings
        assert isinstance(warnings, list)
        for warning in warnings:
            assert set(warning) == {"code", "message"}
            assert warning["code"] and warning["message"]
        # A spectrum with no peaks is flagged, so none goes unmarked.
        if len(spectrum) == 0:
            assert "empty_spectrum" in {w["code"] for w in warnings}


def test_unreadable_scan_blocks_raise(tmp_path):
    path = _fixture_path()
    if not path.is_file():
        if os.environ.get("REQUIRE_CORPUS") == "1":
            pytest.fail(f"REQUIRE_CORPUS=1 but {path} is missing")
        pytest.skip("set OPENSXRAW_CORPUS or OPENSXRAW_TEST_WIFF to a WIFF fixture")
    # Pair the real .wiff with an empty .wiff.scan: every scan block lies
    # past the end of the file, so decoding must raise, not return empty
    # spectra.
    wiff = tmp_path / path.name
    wiff.write_bytes(path.read_bytes())
    (tmp_path / (path.name + ".scan")).write_bytes(b"")
    with pytest.raises(RuntimeError, match="past the end"):
        opensxraw.RawReader(str(wiff))
    with pytest.raises(RuntimeError, match="past the end"):
        next(opensxraw.iter_spectra(str(wiff)))
