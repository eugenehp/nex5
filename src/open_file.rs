use crate::error::{NexError, Result};
use crate::file_data::FileData;
use crate::read_options::ReadOptions;
use crate::reader::Reader;
use crate::stream::{TimestampRange, TimestampStream, validate_timestamp_range};
use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(feature = "mmap")]
use crate::mmap::MmapReader;

enum IoBackend {
    File(File),
    #[cfg(feature = "mmap")]
    Mmap(MmapReader),
}

impl IoBackend {
    #[allow(dead_code)]
    fn file_size(&self) -> u64 {
        match self {
            Self::File(f) => f.metadata().map(|m| m.len()).unwrap_or(0),
            #[cfg(feature = "mmap")]
            Self::Mmap(m) => m.len(),
        }
    }
}
/// An open `.nex` / `.nex5` file with a cached handle for efficient lazy loading.
pub struct OpenNexFile {
    path: PathBuf,
    backend: IoBackend,
    file_size: u64,
    reader: Reader,
    data: FileData,
    is_nex5: bool,
}

impl OpenNexFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_options(path, ReadOptions::default())
    }

    pub fn open_with_options<P: AsRef<Path>>(path: P, options: ReadOptions) -> Result<Self> {
        Reader::with_options(options).open_file(path)
    }

    pub fn open_headers_only<P: AsRef<Path>>(path: P) -> Result<Self> {
        Reader::new().open_headers_only(path)
    }

    /// Open via memory map for zero-copy byte access (`mmap` feature).
    #[cfg(feature = "mmap")]
    pub fn open_mmap<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_mmap_with_options(path, ReadOptions::default())
    }

    #[cfg(feature = "mmap")]
    pub fn open_mmap_with_options<P: AsRef<Path>>(path: P, options: ReadOptions) -> Result<Self> {
        let path = path.as_ref();
        let path_label = path.to_string_lossy();
        let mut backend = MmapReader::open(path).map_err(|e| NexError::io(path_label.as_ref(), e))?;
        let file_size = backend.len();
        let is_nex5 = !matches!(
            path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref(),
            Some("nex")
        );
        let reader = Reader::with_options(options);
        let data = if is_nex5 {
            reader.read_nex5_from(&mut backend, Some(path_label.as_ref()), file_size, None, true)?
        } else {
            reader.read_nex_from(&mut backend, Some(path_label.as_ref()), file_size, None, false)?
        };
        Ok(Self {
            path: path.to_path_buf(),
            backend: IoBackend::Mmap(backend),
            file_size,
            reader,
            data,
            is_nex5,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn data(&self) -> &FileData {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut FileData {
        &mut self.data
    }

    pub fn into_data(self) -> FileData {
        self.data
    }

    pub fn reader(&self) -> &Reader {
        &self.reader
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// Zero-copy view of bytes at `offset` (`mmap` backend only).
    #[cfg(feature = "mmap")]
    pub fn mmap_slice(&self, offset: u64, len: usize) -> Option<&[u8]> {
        match &self.backend {
            IoBackend::Mmap(m) => m.slice_at(offset, len),
            IoBackend::File(_) => None,
        }
    }

    /// Zero-copy decoded layout for a variable payload (`mmap` backend only).
    #[cfg(feature = "mmap")]
    pub fn mmap_variable_view(
        &self,
        name: &str,
    ) -> Result<crate::mmap_view::MmapVariableView<'_>> {
        match &self.backend {
            IoBackend::Mmap(m) => {
                crate::mmap_view::mmap_view_for_variable(m.as_slice(), &self.data, name)
            }
            IoBackend::File(_) => Err(NexError::io(
                self.path.to_string_lossy(),
                "mmap_variable_view requires OpenNexFile::open_mmap",
            )),
        }
    }

    #[cfg(feature = "mmap")]
    pub fn mmap_timestamps_view(
        &self,
        name: &str,
    ) -> Result<crate::mmap_view::MmapTimestampsView<'_>> {
        match self.mmap_variable_view(name)? {
            crate::mmap_view::MmapVariableView::Timestamps(ts) => Ok(ts),
            crate::mmap_view::MmapVariableView::Waveform { timestamps, .. } => Ok(timestamps),
            other => Err(NexError::WrongVariableType(
                other.variable_name_hint(name),
                "neuron, event, marker, or waveform",
            )),
        }
    }

    pub fn load_variables(&mut self, var_names: &[impl AsRef<str>]) -> Result<()> {
        let path_label = self.path.to_string_lossy();
        match &mut self.backend {
            IoBackend::File(file) => self.reader.load_variables_on_handle(
                file,
                self.file_size,
                path_label.as_ref(),
                &mut self.data,
                var_names,
            ),
            #[cfg(feature = "mmap")]
            IoBackend::Mmap(mmap) => self.reader.load_variables_on_handle(
                mmap,
                self.file_size,
                path_label.as_ref(),
                &mut self.data,
                var_names,
            ),
        }
    }

    /// Load only the first `n` timestamps for a spike/event variable.
    pub fn load_timestamps_first(&mut self, var_name: &str, n: usize) -> Result<Vec<f64>> {
        let path_label = self.path.to_string_lossy();
        let idx = self.data.index_of(var_name)?;
        let header = self.data.variables[idx].header().clone();
        let ts = match &mut self.backend {
            IoBackend::File(file) => TimestampStream::new(
                file,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .read_first(n),
            #[cfg(feature = "mmap")]
            IoBackend::Mmap(mmap) => TimestampStream::new(
                mmap,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .read_first(n),
        }?;
        self.apply_timestamps_to_variable(idx, ts)
    }

    /// Load a timestamp sub-range without reading the full spike train.
    pub fn load_timestamps_range(
        &mut self,
        var_name: &str,
        range: TimestampRange,
    ) -> Result<Vec<f64>> {
        let path_label = self.path.to_string_lossy();
        let idx = self.data.index_of(var_name)?;
        let header = self.data.variables[idx].header().clone();
        validate_timestamp_range(&header, range)?;
        let ts = match &mut self.backend {
            IoBackend::File(file) => TimestampStream::new(
                file,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .read_range(range),
            #[cfg(feature = "mmap")]
            IoBackend::Mmap(mmap) => TimestampStream::new(
                mmap,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .read_range(range),
        }?;
        self.apply_timestamps_to_variable(idx, ts)
    }

    /// Stream timestamps through a callback without storing them on `FileData`.
    pub fn stream_timestamps(
        &mut self,
        var_name: &str,
        mut callback: impl FnMut(&[f64]) -> Result<()>,
    ) -> Result<()> {
        let path_label = self.path.to_string_lossy();
        let idx = self.data.index_of(var_name)?;
        let header = self.data.variables[idx].header().clone();
        match &mut self.backend {
            IoBackend::File(file) => TimestampStream::new(
                file,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .for_each(&mut callback),
            #[cfg(feature = "mmap")]
            IoBackend::Mmap(mmap) => TimestampStream::new(
                mmap,
                &header,
                self.data.timestamp_frequency_hz,
                self.reader.options(),
            )
            .with_path_label(Some(path_label.as_ref()))
            .for_each(&mut callback),
        }
    }

    fn apply_timestamps_to_variable(&mut self, idx: usize, ts: Vec<f64>) -> Result<Vec<f64>> {
        use crate::variables::{Timestamps, Variable};
        match &mut self.data.variables[idx] {
            Variable::Event(v) => v.timestamps = Timestamps::from(ts.clone()),
            Variable::Neuron(v) => v.timestamps = Timestamps::from(ts.clone()),
            Variable::Marker(v) => v.timestamps = Timestamps::from(ts.clone()),
            Variable::Waveform(v) => v.timestamps = Timestamps::from(ts.clone()),
            other => {
                return Err(NexError::WrongVariableType(
                    other.name().to_string(),
                    "event, neuron, marker, or waveform",
                ));
            }
        }
        Ok(ts)
    }

    pub(crate) fn from_parts(
        path: PathBuf,
        file: File,
        file_size: u64,
        reader: Reader,
        data: FileData,
        is_nex5: bool,
    ) -> Self {
        Self {
            path,
            backend: IoBackend::File(file),
            file_size,
            reader,
            data,
            is_nex5,
        }
    }
}

impl std::fmt::Debug for OpenNexFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenNexFile")
            .field("path", &self.path)
            .field("variables", &self.data.variables.len())
            .field("is_nex5", &self.is_nex5)
            .field("file_size", &self.file_size)
            .finish()
    }
}
