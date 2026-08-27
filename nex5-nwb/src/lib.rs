//! Read and write NWB 2.x files as [`nex5file::FileData`].
//!
//! Spike trains map to the NWB `Units` table; events and continuous variables
//! map to `acquisition/` [`TimeSeries`](consus_nwb::model::TimeSeries) groups.
//! For spike sorting, PSTHs, and filtering, use the sibling [`nex5-analyze`] crate.
//!
//! [`nex5-analyze`]: ../nex5_analyze/index.html

mod convert;
mod error;
mod metadata;
mod options;

pub use error::{NwbError, Result};
pub use options::{NwbReadOptions, NwbWriteOptions};

use std::fs;
use std::path::Path;

use convert::{file_data_to_nwb_bytes, nwb_bytes_to_file_data};
use nex5file::FileData;

/// Read an NWB file from disk into [`FileData`].
pub fn read_nwb_file(path: impl AsRef<Path>, options: &NwbReadOptions) -> Result<FileData> {
    let bytes = fs::read(path)?;
    read_nwb_bytes(&bytes, options)
}

/// Read NWB HDF5 bytes into [`FileData`].
pub fn read_nwb_bytes(bytes: &[u8], options: &NwbReadOptions) -> Result<FileData> {
    nwb_bytes_to_file_data(bytes, options)
}

/// Write [`FileData`] to an NWB file on disk.
pub fn write_nwb_file(
    data: &FileData,
    path: impl AsRef<Path>,
    options: &NwbWriteOptions,
) -> Result<()> {
    let bytes = write_nwb_bytes(data, options)?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Serialize [`FileData`] to NWB HDF5 bytes.
pub fn write_nwb_bytes(data: &FileData, options: &NwbWriteOptions) -> Result<Vec<u8>> {
    file_data_to_nwb_bytes(data, options)
}
