// PyO3 bindings for the opensxraw library.

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;

use ::openmassspec_core::SpectrumSource;
use ::opensxraw::reader::Reader;

fn to_py_err(e: ::opensxraw::Error) -> PyErr {
    PyRuntimeError::new_err(format!("{e}"))
}

#[pyclass]
pub struct Spectrum {
    #[pyo3(get)]
    pub mz: Vec<f64>,
    #[pyo3(get)]
    pub intensity: Vec<f32>,
    #[pyo3(get)]
    pub ms_level: u32,
    #[pyo3(get)]
    pub retention_time_sec: f64,
}

#[pymethods]
impl Spectrum {
    fn __len__(&self) -> usize {
        self.mz.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "Spectrum({} peaks, RT {:.2}s)",
            self.mz.len(),
            self.retention_time_sec
        )
    }
}

#[pyclass]
pub struct RawReader {
    stem: String,
    spectra: Vec<::openmassspec_core::SpectrumRecord>,
}

#[pymethods]
impl RawReader {
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let mut reader = Reader::open(path).map_err(to_py_err)?;
        let stem = reader.stem.clone();
        let spectra = reader.iter_spectra().collect();

        Ok(Self { stem, spectra })
    }

    #[getter]
    fn scan_count(&self) -> usize {
        self.spectra.len()
    }

    fn read_spectrum(&self, scan_index: usize) -> PyResult<Spectrum> {
        let spec = self
            .spectra
            .get(scan_index)
            .ok_or_else(|| PyRuntimeError::new_err(format!("scan {} out of range", scan_index)))?;

        Ok(Spectrum {
            mz: spec.mz.clone(),
            intensity: spec.intensity.clone(),
            ms_level: spec.ms_level,
            retention_time_sec: spec.retention_time_sec,
        })
    }

    fn __repr__(&self) -> String {
        format!("RawReader('{}', {} scans)", self.stem, self.spectra.len())
    }
}

#[pymodule]
fn opensxraw(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RawReader>()?;
    m.add_class::<Spectrum>()?;
    Ok(())
}
