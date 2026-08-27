//! Fluent builder for [`FileData`](crate::FileData).

use crate::compat::{String, Vec};
use crate::error::Result;
use crate::file_data::FileData;
use crate::variables::Variable;
use serde_json::Value as JsonValue;

/// Builds a [`FileData`] with a fluent API.
pub struct FileDataBuilder {
    inner: Option<FileData>,
}

impl FileDataBuilder {
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn timestamp_frequency_hz(mut self, hz: f64) -> Result<Self> {
        self.inner = Some(FileData::new(hz, "")?);
        Ok(self)
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        if let Some(data) = &mut self.inner {
            data.comment = comment.into();
        }
        self
    }

    pub fn beg_seconds(mut self, seconds: f64) -> Self {
        if let Some(data) = &mut self.inner {
            data.beg_seconds = seconds;
        }
        self
    }

    pub fn metadata(mut self, metadata: JsonValue) -> Self {
        if let Some(data) = &mut self.inner {
            data.metadata = metadata;
        }
        self
    }

    pub fn event(mut self, name: impl Into<String>, timestamps: Vec<f64>) -> Result<Self> {
        let data = self.inner_mut()?;
        data.add_event(name, timestamps)?;
        Ok(self)
    }

    pub fn neuron(
        mut self,
        name: impl Into<String>,
        timestamps: Vec<f64>,
        wire: i32,
        unit: i32,
        x: f64,
        y: f64,
    ) -> Result<Self> {
        let data = self.inner_mut()?;
        data.add_neuron(name, timestamps, wire, unit, x, y)?;
        Ok(self)
    }

    pub fn push_variable(mut self, var: Variable) -> Result<Self> {
        let data = self.inner_mut()?;
        let name = var.name().to_string();
        if data.get_variable(name.as_str()).is_ok() {
            return Err(crate::error::NexError::DuplicateVariable(name));
        }
        data.variables.push(var);
        data.rebuild_index();
        Ok(self)
    }

    pub fn build(self) -> Result<FileData> {
        self.inner.ok_or(crate::error::NexError::InvalidTimestampFrequency)
    }

    fn inner_mut(&mut self) -> Result<&mut FileData> {
        self.inner
            .as_mut()
            .ok_or(crate::error::NexError::InvalidTimestampFrequency)
    }
}

impl Default for FileDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FileData {
    /// Start a fluent builder (call `.timestamp_frequency_hz(hz)?` first).
    pub fn builder() -> FileDataBuilder {
        FileDataBuilder::new()
    }
}
