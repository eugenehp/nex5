#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::{default_metadata, max_of_slice_or_zero, Timestamps};
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EventVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub timestamps: Timestamps,
}

impl EventVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            timestamps: Timestamps::new(),
        }
    }
}

impl EventVariable {
    pub fn timestamps_copy(&self) -> Vec<f64> {
        self.timestamps.as_f64_vec()
    }

    pub fn maximum_timestamp(&self) -> f64 {
        max_of_slice_or_zero(&self.timestamps.as_f64_vec())
    }

    pub fn count_for_header(&self) -> u64 {
        self.timestamps.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        super::timestamp_payload_bytes(self.timestamps.len(), hw)
    }
}
