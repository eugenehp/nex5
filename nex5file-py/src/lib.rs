use nex5file::{read_nex5_file, write_nex5_file, FileData, Result};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn map_err(err: nex5file::NexError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

#[pyfunction]
fn read_nex5(path: &str) -> PyResult<PyFileData> {
    let data = read_nex5_file(path).map_err(map_err)?;
    Ok(PyFileData { inner: data })
}

#[pyfunction]
fn write_nex5(path: &str, data: &PyFileData) -> PyResult<()> {
    write_nex5_file(&data.inner, path).map_err(map_err)
}

#[pyclass(name = "FileData")]
struct PyFileData {
    inner: FileData,
}

#[pymethods]
impl PyFileData {
    #[new]
    fn new(freq: f64, comment: &str) -> PyResult<Self> {
        Ok(Self {
            inner: FileData::new(freq, comment).map_err(map_err)?,
        })
    }

    fn event_names(&self) -> Vec<String> {
        self.inner.event_names()
    }

    fn neuron_names(&self) -> Vec<String> {
        self.inner.neuron_names()
    }

    fn continuous_names(&self) -> Vec<String> {
        self.inner.continuous_names()
    }

    fn get_event_timestamps(&self, name: &str) -> PyResult<Vec<f64>> {
        Ok(self
            .inner
            .event(name)
            .map_err(map_err)?
            .timestamps
            .as_f64_vec())
    }

    fn get_neuron_timestamps(&self, name: &str) -> PyResult<Vec<f64>> {
        Ok(self
            .inner
            .neuron(name)
            .map_err(map_err)?
            .timestamps
            .as_f64_vec())
    }

    fn add_event(&mut self, name: &str, timestamps: Vec<f64>) -> PyResult<()> {
        self.inner.add_event(name, timestamps).map_err(map_err)
    }

    fn add_neuron(
        &mut self,
        name: &str,
        timestamps: Vec<f64>,
        wire: i32,
        unit: i32,
        x: f64,
        y: f64,
    ) -> PyResult<()> {
        self.inner
            .add_neuron(name, timestamps, wire, unit, x, y)
            .map_err(map_err)
    }
}

#[pymodule]
fn nex5file_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(read_nex5, m)?)?;
    m.add_function(wrap_pyfunction!(write_nex5, m)?)?;
    m.add_class::<PyFileData>()?;
    Ok(())
}
