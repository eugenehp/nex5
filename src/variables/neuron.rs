#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::Timestamps;
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NeuronVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub timestamps: Timestamps,
}

impl NeuronVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        let metadata = if header.version < 500 {
            serde_json::json!({
                "name": name,
                "nameOriginal": name,
                "unitNumber": header.unit,
                "probe": {
                    "position": { "x": header.x_pos, "y": header.y_pos },
                    "wireNumber": header.wire,
                },
            })
        } else {
            serde_json::json!({
                "name": name,
                "nameOriginal": name,
                "unitNumber": 0,
                "probe": {
                    "position": { "x": 0.0, "y": 0.0 },
                    "wireNumber": 0,
                },
            })
        };

        Self {
            header,
            metadata,
            timestamps: Timestamps::new(),
        }
    }

    pub fn timestamps_copy(&self) -> Vec<f64> {
        self.timestamps.as_f64_vec()
    }

    pub fn maximum_timestamp(&self) -> f64 {
        super::max_of_slice_or_zero(&self.timestamps.as_f64_vec())
    }

    pub fn count_for_header(&self) -> u64 {
        self.timestamps.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        super::timestamp_payload_bytes(self.timestamps.len(), hw)
    }

    pub fn assign_from_var_meta(&mut self) {
        if let Some(unit) = self.metadata.get("unitNumber").and_then(|v| v.as_i64()) {
            self.header.unit = unit as i32;
        }
        if let Some(wire) = self
            .metadata
            .pointer("/probe/wireNumber")
            .and_then(|v| v.as_i64())
        {
            self.header.wire = wire as i32;
        }
        if let Some(x) = self
            .metadata
            .pointer("/probe/position/x")
            .and_then(|v| v.as_f64())
        {
            self.header.x_pos = x;
        }
        if let Some(y) = self
            .metadata
            .pointer("/probe/position/y")
            .and_then(|v| v.as_f64())
        {
            self.header.y_pos = y;
        }
    }
}
