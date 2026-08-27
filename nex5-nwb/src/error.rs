use thiserror::Error;

#[derive(Debug, Error)]
pub enum NwbError {
    #[error("NWB HDF5 error: {0}")]
    Hdf5(String),

    #[error("nex5 error: {0}")]
    Nex(#[from] nex5file::NexError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = core::result::Result<T, NwbError>;

pub(crate) fn from_consus(err: consus_core::Error) -> NwbError {
    NwbError::Hdf5(format!("{err:?}"))
}
