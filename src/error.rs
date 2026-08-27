use crate::compat::{String, ToString};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NexError {
    #[error("invalid .nex file header")]
    InvalidNexHeader,

    #[error("invalid .nex5 file header")]
    InvalidNex5Header,

    #[error("unable to read all values")]
    IncompleteRead,

    #[error("invalid timestamp frequency")]
    InvalidTimestampFrequency,

    #[error("unable to find variable \"{0}\" in file data")]
    VariableNotFound(String),

    #[error("unable to add variable: variable is invalid")]
    InvalidVariable,

    #[error("unable to add variable with name \"{0}\": variable with this name already exists")]
    DuplicateVariable(String),

    #[error("invalid marker parameters")]
    InvalidMarkerParameters,

    #[error("invalid sampling rate: the rate should be positive and less than FileData timestamp frequency")]
    InvalidSamplingRate,

    #[error("invalid timestamps and values (both arrays should be the same length)")]
    InvalidTimestampsAndValues,

    #[error("invalid waveform values matrix")]
    InvalidWaveformValues,

    #[error("marker values should be either all numbers or all strings")]
    InvalidMarkerValues,

    #[error("string marker value in numeric marker mode: {0}")]
    MarkerStringInNumericMode(String),

    #[error("unknown variable type {0} for variable \"{1}\"")]
    UnknownVariableType(i32, String),

    #[error("invalid waveform header: NPointsWave is not positive\n{0}")]
    InvalidWaveformHeaderNPoints(String),

    #[error("invalid waveform header: SamplingRate is not positive\n{0}")]
    InvalidWaveformHeaderSamplingRate(String),

    #[error("invalid continuous header: SamplingRate is not positive\n{0}")]
    InvalidContinuousHeader(String),

    #[error("invalid data offset {offset} for variable \"{name}\" (file size {file_size})")]
    InvalidDataOffset {
        name: String,
        offset: u64,
        file_size: u64,
    },

    #[error("payload size {requested} exceeds limit {limit} for variable \"{name}\"")]
    PayloadTooLarge {
        name: String,
        requested: u64,
        limit: u64,
    },

    #[error(
        "unable to save FileData object if all variables have no data. NeuroExplorer will reject .nex file with no data."
    )]
    NoVariableData,

    #[error(
        "unable to save as .nex file: max timestamp exceeds 32-bit range; you can save as .nex5 file instead"
    )]
    TimestampExceeds32Bit,

    #[error("wrong variable type for \"{0}\": expected {1}")]
    WrongVariableType(String, &'static str),

    #[error("variable \"{0}\" data is not loaded; call load_variables first")]
    VariableNotLoaded(String),

    #[error("invalid interval: end time before start time at index {index}")]
    InvalidInterval { index: usize },

    #[error("invalid file metadata: {0}")]
    InvalidMetadata(String),

    #[error("I/O error at {path}: {message}")]
    Io { path: String, message: String },
}

pub type Result<T> = core::result::Result<T, NexError>;

impl NexError {
    pub fn io(path: impl Into<String>, source: impl core::fmt::Display) -> Self {
        Self::Io {
            path: path.into(),
            message: source.to_string(),
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for NexError {
    fn from(source: std::io::Error) -> Self {
        Self::io("<unknown>", source)
    }
}

#[cfg(not(feature = "std"))]
impl From<crate::io_ext::Error> for NexError {
    fn from(source: crate::io_ext::Error) -> Self {
        Self::io("<unknown>", source)
    }
}

pub(crate) fn map_io_err(path: Option<&str>, source: impl core::fmt::Display) -> NexError {
    NexError::io(path.unwrap_or("<reader>"), source)
}
