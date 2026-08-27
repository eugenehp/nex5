use thiserror::Error;

#[derive(Debug, Error)]
pub enum SortError {
    #[error("nex5 error: {0}")]
    Nex(#[from] nex5file::NexError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("npy error: {0}")]
    Npy(String),

    #[error("phy/kilosort error: {0}")]
    Phy(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = core::result::Result<T, SortError>;
