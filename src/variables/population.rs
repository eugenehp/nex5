#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::default_metadata;
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PopulationVector {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub weights: Vec<f64>,
}

impl PopulationVector {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            weights: Vec::new(),
        }
    }

    pub fn weights_copy(&self) -> Vec<f64> {
        self.weights.clone()
    }

    pub fn maximum_timestamp(&self) -> f64 {
        0.0
    }

    pub fn count_for_header(&self) -> u64 {
        self.weights.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, _hw: &VariableHeader) -> u64 {
        (self.weights.len() * 8) as u64
    }
}
