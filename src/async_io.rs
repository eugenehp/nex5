//! Async file I/O wrappers (`async` feature).

use crate::error::{NexError, Result};
use crate::file_data::FileData;
use crate::read_options::ReadOptions;
use crate::reader::{NexFormat, Reader};
use crate::write_options::WriteOptions;
use crate::writer::Writer;
use std::path::Path;

/// Read a `.nex` or `.nex5` file asynchronously (reads bytes on async runtime, parses synchronously).
pub async fn read_nex5_file_async(path: impl AsRef<Path>) -> Result<FileData> {
    read_nex_file_async(path).await
}

pub async fn read_nex_file_async(path: impl AsRef<Path>) -> Result<FileData> {
    read_with_options_async(path, ReadOptions::default()).await
}

pub async fn read_with_options_async(
    path: impl AsRef<Path>,
    options: ReadOptions,
) -> Result<FileData> {
    let path = path.as_ref();
    let path_label = path.to_string_lossy().into_owned();
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|e| NexError::io(&path_label, e))?;
    let format = match path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("nex") => NexFormat::Nex,
        _ => NexFormat::Nex5,
    };
    Reader::with_options(options).read_from_slice(&bytes, format)
}

pub async fn write_with_options_async(
    path: impl AsRef<Path>,
    data: &FileData,
    options: WriteOptions,
    format: NexFormat,
) -> Result<()> {
    let path = path.as_ref();
    let path_label = path.to_string_lossy().into_owned();
    let bytes = Writer::with_options(options).write_to_vec(data, format)?;
    tokio::fs::write(path, bytes)
        .await
        .map_err(|e| NexError::io(&path_label, e))
}

pub async fn write_nex5_file_async(path: impl AsRef<Path>, data: &FileData) -> Result<()> {
    write_with_options_async(path, data, WriteOptions::default(), NexFormat::Nex5).await
}

pub async fn write_nex_file_async(path: impl AsRef<Path>, data: &FileData) -> Result<()> {
    write_with_options_async(path, data, WriteOptions::default(), NexFormat::Nex).await
}
