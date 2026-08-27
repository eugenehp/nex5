#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

/// Options controlling file write behavior.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Buffered writer capacity in bytes.
    pub buffer_bytes: usize,
    /// Software name recorded in `.nex5` JSON metadata.
    pub writer_name: String,
    /// Software version recorded in `.nex5` JSON metadata.
    pub writer_version: String,
    /// Write trailing JSON metadata block (`.nex5` only).
    pub embed_metadata: bool,
}

impl Default for WriteOptions {
    fn default() -> Self {
        Self {
            buffer_bytes: 256 * 1024,
            writer_name: "nex5file".to_string(),
            writer_version: env!("CARGO_PKG_VERSION").to_string(),
            embed_metadata: true,
        }
    }
}

impl WriteOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn buffer_bytes(mut self, bytes: usize) -> Self {
        self.buffer_bytes = bytes;
        self
    }

    pub fn writer_name(mut self, name: impl Into<String>) -> Self {
        self.writer_name = name.into();
        self
    }

    pub fn writer_version(mut self, version: impl Into<String>) -> Self {
        self.writer_version = version.into();
        self
    }

    pub fn embed_metadata(mut self, embed: bool) -> Self {
        self.embed_metadata = embed;
        self
    }
}
