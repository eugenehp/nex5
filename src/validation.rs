#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use crate::error::{NexError, Result};
use crate::format::VariableHeader;
use crate::read_options::ReadOptions;
use crate::variables::NexFileVarType;

pub fn verify_variable_header(header: &VariableHeader) -> Result<()> {
    if NexFileVarType::from_i32(header.var_type).is_none() {
        return Err(NexError::UnknownVariableType(
            header.var_type,
            header.name.clone(),
        ));
    }
    if header.var_type == NexFileVarType::Waveform as i32 && header.n_points_wave == 0 {
        return Err(NexError::InvalidWaveformHeaderNPoints(header.to_string()));
    }
    if header.var_type == NexFileVarType::Waveform as i32 && header.sampling_rate <= 0.0 {
        return Err(NexError::InvalidWaveformHeaderSamplingRate(
            header.to_string(),
        ));
    }
    if header.var_type == NexFileVarType::Continuous as i32 && header.sampling_rate <= 0.0 {
        return Err(NexError::InvalidContinuousHeader(header.to_string()));
    }
    Ok(())
}

pub fn validate_variable_layout(
    header: &VariableHeader,
    file_size: u64,
    options: &ReadOptions,
) -> Result<()> {
    if !options.validate_layout {
        return Ok(());
    }

    if header.data_offset >= file_size && header.count > 0 {
        return Err(NexError::InvalidDataOffset {
            name: header.name.clone(),
            offset: header.data_offset,
            file_size,
        });
    }

    let payload = estimated_payload_bytes(header);
    if options.max_payload_bytes > 0 && payload > options.max_payload_bytes {
        return Err(NexError::PayloadTooLarge {
            name: header.name.clone(),
            requested: payload,
            limit: options.max_payload_bytes,
        });
    }

    if header.data_offset.saturating_add(payload) > file_size.saturating_add(1) {
        return Err(NexError::InvalidDataOffset {
            name: header.name.clone(),
            offset: header.data_offset,
            file_size,
        });
    }

    Ok(())
}

pub fn validate_intervals(starts: &[f64], ends: &[f64]) -> Result<()> {
    if starts.len() != ends.len() {
        return Err(NexError::InvalidTimestampsAndValues);
    }
    for (i, (&start, &end)) in starts.iter().zip(ends.iter()).enumerate() {
        if end < start {
            return Err(NexError::InvalidInterval { index: i });
        }
    }
    Ok(())
}

pub fn estimated_payload_bytes(header: &VariableHeader) -> u64 {
    let ts_bytes = header.bytes_in_timestamp() as u64;
    let count = header.count;

    match NexFileVarType::from_i32(header.var_type) {
        Some(NexFileVarType::Event | NexFileVarType::Neuron) => ts_bytes * count,
        Some(NexFileVarType::Interval) => ts_bytes * count * 2,
        Some(NexFileVarType::Marker) => {
            ts_bytes * count
                + (header.n_markers as u64) * (64 + header.marker_length as u64 * count)
        }
        Some(NexFileVarType::Continuous) => {
            ts_bytes * count
                + 4 * count
                + header.bytes_in_cont_value() as u64 * header.n_points_wave
        }
        Some(NexFileVarType::Waveform) => {
            ts_bytes * count + header.bytes_in_cont_value() as u64 * count * header.n_points_wave
        }
        Some(NexFileVarType::PopulationVector) => 8 * count,
        None => 0,
    }
}
