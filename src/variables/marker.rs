#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::{default_metadata, max_of_slice_or_zero, Timestamps};
use crate::error::{NexError, Result};
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MarkerFieldValue {
    Number(u32),
    String(String),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MarkerVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub timestamps: Timestamps,
    pub marker_field_names: Vec<String>,
    pub marker_fields: Vec<Vec<MarkerFieldValue>>,
}

impl MarkerVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            timestamps: Timestamps::new(),
            marker_field_names: Vec::new(),
            marker_fields: Vec::new(),
        }
    }

    pub fn timestamps_copy(&self) -> Vec<f64> {
        self.timestamps.as_f64_vec()
    }

    pub fn marker_field_names_copy(&self) -> Vec<String> {
        self.marker_field_names.clone()
    }

    pub fn markers_copy(&self) -> Vec<Vec<MarkerFieldValue>> {
        self.marker_fields.clone()
    }

    pub fn maximum_timestamp(&self) -> f64 {
        max_of_slice_or_zero(&self.timestamps.as_f64_vec())
    }

    pub fn count_for_header(&self) -> u64 {
        self.timestamps.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        let num_markers = self.timestamps.len();
        let mut total = super::timestamp_payload_bytes(num_markers, hw);
        total += (self.marker_field_names.len() as u64)
            * (64 + hw.marker_length as u64 * num_markers as u64);
        total
    }

    pub fn all_marker_values_are_numeric(&self) -> bool {
        self.marker_fields
            .iter()
            .flatten()
            .all(|v| matches!(v, MarkerFieldValue::Number(_)))
    }

    pub fn if_number_strings_store_as_numbers(&mut self) {
        if !self.all_marker_values_are_numeric() {
            for field in &mut self.marker_fields {
                for value in field.iter_mut() {
                    if let MarkerFieldValue::String(s) = value {
                        if let Ok(n) = s.parse::<u32>() {
                            *value = MarkerFieldValue::Number(n);
                        }
                    }
                }
            }
        }
    }

    pub fn calc_marker_length(&mut self) -> crate::error::Result<()> {
        self.calc_marker_length_checked()
    }

    pub fn calc_marker_length_checked(&mut self) -> Result<()> {
        self.header.n_markers = self.marker_field_names.len() as i32;
        if self.all_marker_values_are_numeric() {
            self.header.marker_length = 6;
            self.header.marker_data_type = 1;
            Ok(())
        } else {
            self.header.marker_data_type = 0;
            let mut max_string_length = 0usize;
            for field in &self.marker_fields {
                for value in field {
                    match value {
                        MarkerFieldValue::String(s) => {
                            max_string_length = max_string_length.max(s.len());
                        }
                        MarkerFieldValue::Number(_) => return Err(NexError::InvalidMarkerValues),
                    }
                }
            }
            self.header.marker_length = (max_string_length + 1).max(6) as i32;
            Ok(())
        }
    }
}
