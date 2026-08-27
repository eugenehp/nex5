use super::{default_metadata, max_of_slice_or_zero, Timestamps};
use crate::compat::{format, String, Vec};
use crate::error::{NexError, Result};
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;
use sha1::{Digest, Sha1};

/// Waveform variable with flat sample storage (`count × n_points` values, row-major).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WaveformVariable {
    pub header: VariableHeader,
    pub metadata: JsonValue,
    pub timestamps: Timestamps,
    pub waveform_values: Vec<f32>,
    pub hashed_wave_values: String,
}

impl WaveformVariable {
    pub fn new(header: VariableHeader) -> Self {
        let name = header.name.clone();
        Self {
            header,
            metadata: default_metadata(&name),
            timestamps: Timestamps::new(),
            waveform_values: Vec::new(),
            hashed_wave_values: String::new(),
        }
    }

    pub fn sampling_rate(&self) -> f64 {
        self.header.sampling_rate
    }

    pub fn num_points_in_wave(&self) -> u64 {
        self.header.n_points_wave
    }

    pub fn timestamps_copy(&self) -> Vec<f64> {
        self.timestamps.as_f64_vec()
    }

    /// View of one waveform's samples.
    pub fn wave(&self, index: usize) -> Option<&[f32]> {
        let n = self.header.n_points_wave as usize;
        if n == 0 {
            return None;
        }
        let start = index.checked_mul(n)?;
        self.waveform_values.get(start..start + n)
    }

    /// Nested view (allocates); prefer [`wave`](Self::wave) or [`waveform_values`](Self::waveform_values).
    pub fn waveform_values_nested(&self) -> Vec<Vec<f32>> {
        let n = self.header.n_points_wave as usize;
        if n == 0 {
            return Vec::new();
        }
        self.waveform_values
            .chunks(n)
            .map(<[f32]>::to_vec)
            .collect()
    }

    pub fn pre_threshold_time(&self) -> f64 {
        self.header.pre_thr_time
    }

    pub fn maximum_timestamp(&self) -> f64 {
        if self.timestamps.is_empty() {
            return 0.0;
        }
        max_of_slice_or_zero(&self.timestamps.as_f64_vec())
            + (self.header.n_points_wave as f64 - 1.0) / self.header.sampling_rate
    }

    pub fn assign_num_points_wave(&mut self) -> Result<()> {
        if self.timestamps.is_empty() {
            return Ok(());
        }
        let n = self.header.n_points_wave as usize;
        if n == 0 || self.waveform_values.len() != self.timestamps.len() * n {
            return Err(NexError::InvalidWaveformValues);
        }
        Ok(())
    }

    pub fn set_from_nested(&mut self, nested: Vec<Vec<f32>>) -> Result<()> {
        if nested.is_empty() {
            self.waveform_values.clear();
            self.header.n_points_wave = 0;
            return Ok(());
        }
        let n = nested[0].len();
        if n == 0 || nested.iter().any(|w| w.len() != n) {
            return Err(NexError::InvalidWaveformValues);
        }
        self.header.n_points_wave = n as u64;
        self.waveform_values = nested.into_iter().flatten().collect();
        Ok(())
    }

    pub fn hash_cont_values(&mut self) {
        self.hashed_wave_values = hex_hash_f32(&self.waveform_values);
    }

    pub fn cont_scaling(&self, cont_data_type: i32) -> (f64, f64) {
        if cont_data_type == 1 || self.timestamps.is_empty() {
            return (1.0, 0.0);
        }

        if !self.hashed_wave_values.is_empty() {
            let hash = hex_hash_f32(&self.waveform_values);
            if hash == self.hashed_wave_values {
                return (self.header.ad_to_mv, self.header.mv_offset);
            }
        }

        let cont_max = self
            .waveform_values
            .iter()
            .map(|v| v.abs())
            .fold(0.0f32, f32::max) as f64;

        if cont_max == 0.0 {
            (1.0, 0.0)
        } else {
            (cont_max / 32767.0, 0.0)
        }
    }

    pub fn waveform_values_flat(&self) -> Vec<f32> {
        self.waveform_values.clone()
    }

    pub fn set_from_flat(&mut self, flat: Vec<f32>, n_points: usize) -> Result<()> {
        if n_points == 0 || flat.len() % n_points != 0 {
            return Err(NexError::InvalidWaveformValues);
        }
        self.waveform_values = flat;
        self.header.n_points_wave = n_points as u64;
        Ok(())
    }

    pub fn count_for_header(&self) -> u64 {
        self.timestamps.len() as u64
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        (hw.bytes_in_timestamp() * self.timestamps.len()
            + hw.bytes_in_cont_value() * self.waveform_values.len()) as u64
    }
}

fn hex_hash_f32(values: &[f32]) -> String {
    let mut hasher = Sha1::new();
    for &v in values {
        hasher.update(v.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}
