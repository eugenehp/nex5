use crate::compat::round_f64;

#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use super::default_metadata;
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;
use sha1::{Digest, Sha1};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContinuousVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub fragment_timestamps: Vec<f64>,
    pub fragment_indexes: Vec<u32>,
    pub fragment_counts: Vec<i64>,
    pub continuous_values: Vec<f64>,
    pub hashed_cont_values: String,
}

impl ContinuousVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            fragment_timestamps: Vec::new(),
            fragment_indexes: Vec::new(),
            fragment_counts: Vec::new(),
            continuous_values: Vec::new(),
            hashed_cont_values: String::new(),
        }
    }

    pub fn sampling_rate(&self) -> f64 {
        self.header.sampling_rate
    }

    pub fn continuous_values_copy(&self) -> Vec<f64> {
        self.continuous_values.clone()
    }

    pub fn fragment_timestamps_copy(&self) -> Vec<f64> {
        self.fragment_timestamps.clone()
    }

    pub fn fragment_counts_copy(&self) -> Vec<i64> {
        self.fragment_counts.clone()
    }

    pub fn hash_cont_values(&mut self) {
        let bytes = continuous_values_as_bytes(&self.continuous_values);
        self.hashed_cont_values = hex_hash(&bytes);
    }

    pub fn calculate_fragments_from_all_timestamps(&mut self, ts_freq: f64) {
        if self.fragment_timestamps.len() < 2 {
            return;
        }

        let max_diff_to_consolidate = 0.000_001;
        let sr = self.header.sampling_rate;
        if (round_f64(ts_freq / sr) - (ts_freq / sr)).abs() > max_diff_to_consolidate {
            return;
        }

        let step = round_f64(ts_freq / sr) as i64;
        let ts_in_ticks: Vec<i64> = self
            .fragment_timestamps
            .iter()
            .map(|t| round_f64(t * ts_freq) as i64)
            .collect();

        let mut new_fragment_timestamps = vec![self.fragment_timestamps[0]];
        let mut new_fragment_starts = vec![0u32];
        let mut data_point_index = 0usize;
        let mut expected_timestamp_of_next_fragment = ts_in_ticks[0] + step;

        for (i, ts) in ts_in_ticks.iter().enumerate().skip(1) {
            data_point_index += 1;
            if *ts - expected_timestamp_of_next_fragment != 0 {
                new_fragment_timestamps.push(self.fragment_timestamps[i]);
                new_fragment_starts.push(data_point_index as u32);
            }
            expected_timestamp_of_next_fragment = *ts + step;
        }

        self.fragment_timestamps = new_fragment_timestamps;
        self.fragment_indexes = new_fragment_starts;
        self.calculate_fragment_counts_from_indexes();
    }

    pub fn calculate_fragment_counts_from_indexes(&mut self) {
        let mut fragment_counts = Vec::with_capacity(self.fragment_indexes.len());
        for frag in 0..self.fragment_indexes.len() {
            let count = if frag < self.fragment_indexes.len() - 1 {
                self.fragment_indexes[frag + 1] - self.fragment_indexes[frag]
            } else {
                self.continuous_values.len() as u32 - self.fragment_indexes[frag]
            };
            fragment_counts.push(count as i64);
        }
        self.fragment_counts = fragment_counts;
    }

    pub fn maximum_timestamp(&self) -> f64 {
        if self.fragment_timestamps.is_empty() {
            return 0.0;
        }
        let last_frag = self.fragment_timestamps.len() - 1;
        self.fragment_timestamps[last_frag]
            + (self.fragment_counts[last_frag] as f64 - 1.0) / self.header.sampling_rate
    }

    pub fn cont_scaling(&self, cont_data_type: i32) -> (f64, f64) {
        if cont_data_type == 1 || self.continuous_values.is_empty() {
            return (1.0, 0.0);
        }

        if !self.hashed_cont_values.is_empty() {
            let bytes = continuous_values_as_bytes(&self.continuous_values);
            if hex_hash(&bytes) == self.hashed_cont_values {
                return (self.header.ad_to_mv, self.header.mv_offset);
            }
        }

        let cont_max = self
            .continuous_values
            .iter()
            .map(|v| v.abs())
            .fold(0.0f64, f64::max);

        if cont_max == 0.0 {
            (1.0, 0.0)
        } else {
            (cont_max / 32767.0, 0.0)
        }
    }

    pub fn count_for_header(&self) -> u64 {
        self.fragment_timestamps.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        let n = self.fragment_timestamps.len();
        ((hw.bytes_in_timestamp() + hw.bytes_in_fragment_index()) * n
            + hw.bytes_in_cont_value() * self.continuous_values.len()) as u64
    }
}

fn continuous_values_as_bytes(values: &[f64]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn hex_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
