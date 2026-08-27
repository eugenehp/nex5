//! Typed accessors for `.nex5` JSON metadata.

use crate::compat::{String, ToString, Vec};
use crate::file_data::FileData;
use serde_json::Value as JsonValue;

/// File-level metadata block from a `.nex5` file.
#[derive(Debug, Clone, Default)]
pub struct FileMetadata {
    pub writer_name: Option<String>,
    pub writer_version: Option<String>,
    pub variables: Vec<VariableMetadata>,
    pub raw: JsonValue,
}

/// Per-variable metadata entry inside the file JSON block.
#[derive(Debug, Clone, Default)]
pub struct VariableMetadata {
    pub name: String,
    pub unit_number: Option<i32>,
    pub wire_number: Option<i32>,
    pub x_pos: Option<f64>,
    pub y_pos: Option<f64>,
    pub raw: JsonValue,
}

impl FileMetadata {
    pub fn from_json(value: &JsonValue) -> Self {
        let mut meta = Self {
            raw: value.clone(),
            ..Default::default()
        };
        if let Some(file) = value.get("file") {
            if let Some(ws) = file.get("writerSoftware") {
                meta.writer_name = ws
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                meta.writer_version = ws
                    .get("version")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
            }
        }
        if let Some(vars) = value.get("variables").and_then(|v| v.as_array()) {
            meta.variables = vars.iter().map(VariableMetadata::from_json).collect();
        }
        meta
    }

    pub fn variable_by_name(&self, name: &str) -> Option<&VariableMetadata> {
        self.variables.iter().find(|v| v.name == name)
    }
}

impl VariableMetadata {
    pub fn from_json(value: &JsonValue) -> Self {
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let unit_number = value.get("unitNumber").and_then(|v| v.as_i64()).map(|n| n as i32);
        let (wire_number, x_pos, y_pos) = value
            .get("probe")
            .map(|probe| {
                let wire = probe.get("wireNumber").and_then(|v| v.as_i64()).map(|n| n as i32);
                let x = probe
                    .get("position")
                    .and_then(|p| p.get("x"))
                    .and_then(|v| v.as_f64());
                let y = probe
                    .get("position")
                    .and_then(|p| p.get("y"))
                    .and_then(|v| v.as_f64());
                (wire, x, y)
            })
            .unwrap_or((None, None, None));
        Self {
            name,
            unit_number,
            wire_number,
            x_pos,
            y_pos,
            raw: value.clone(),
        }
    }
}

impl FileData {
    /// Parsed view of [`FileData::metadata`].
    pub fn file_metadata(&self) -> FileMetadata {
        FileMetadata::from_json(&self.metadata)
    }

    /// Metadata for one variable by name (from JSON block).
    pub fn variable_metadata(&self, name: &str) -> Option<VariableMetadata> {
        self.file_metadata().variable_by_name(name).cloned()
    }
}
