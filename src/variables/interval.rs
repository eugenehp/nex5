#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::{default_metadata, max_of_slice_or_zero};
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IntervalVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub interval_starts: Vec<f64>,
    pub interval_ends: Vec<f64>,
}

impl IntervalVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            interval_starts: Vec::new(),
            interval_ends: Vec::new(),
        }
    }

    pub fn intervals(&self) -> (Vec<f64>, Vec<f64>) {
        (self.interval_starts.clone(), self.interval_ends.clone())
    }

    pub fn maximum_timestamp(&self) -> f64 {
        max_of_slice_or_zero(&self.interval_ends)
    }

    pub fn count_for_header(&self) -> u64 {
        self.interval_starts.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        super::timestamp_payload_bytes(self.interval_starts.len(), hw) * 2
    }
}
