#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

mod continuous;
mod event;
mod timestamps;
mod interval;
mod marker;
mod neuron;
mod population;
mod waveform;

pub use continuous::ContinuousVariable;
pub use event::EventVariable;
pub use timestamps::Timestamps;
pub use interval::IntervalVariable;
pub use marker::{MarkerFieldValue, MarkerVariable};
pub use neuron::NeuronVariable;
pub use population::PopulationVector;
pub use waveform::WaveformVariable;

use crate::error::{NexError, Result};
use crate::format::VariableHeader;
use serde_json::Value as JsonValue;

/// Variable type constants matching NeuroExplorer .nex / .nex5 format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(i32)]
pub enum NexFileVarType {
    Neuron = 0,
    Event = 1,
    Interval = 2,
    Waveform = 3,
    PopulationVector = 4,
    Continuous = 5,
    Marker = 6,
}

impl NexFileVarType {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(Self::Neuron),
            1 => Some(Self::Event),
            2 => Some(Self::Interval),
            3 => Some(Self::Waveform),
            4 => Some(Self::PopulationVector),
            5 => Some(Self::Continuous),
            6 => Some(Self::Marker),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Neuron => "neuron",
            Self::Event => "event",
            Self::Interval => "interval",
            Self::Waveform => "waveform",
            Self::PopulationVector => "population_vector",
            Self::Continuous => "continuous",
            Self::Marker => "marker",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Variable {
    Neuron(NeuronVariable),
    Event(EventVariable),
    Interval(IntervalVariable),
    Marker(MarkerVariable),
    Waveform(WaveformVariable),
    Continuous(ContinuousVariable),
    PopulationVector(PopulationVector),
}

impl Variable {
    pub fn try_from_header(header: VariableHeader) -> Result<Self> {
        Ok(match NexFileVarType::from_i32(header.var_type) {
            Some(NexFileVarType::Neuron) => Self::Neuron(NeuronVariable::new(header)),
            Some(NexFileVarType::Event) => Self::Event(EventVariable::new(header)),
            Some(NexFileVarType::Interval) => Self::Interval(IntervalVariable::new(header)),
            Some(NexFileVarType::Marker) => Self::Marker(MarkerVariable::new(header)),
            Some(NexFileVarType::Waveform) => Self::Waveform(WaveformVariable::new(header)),
            Some(NexFileVarType::Continuous) => Self::Continuous(ContinuousVariable::new(header)),
            Some(NexFileVarType::PopulationVector) => {
                Self::PopulationVector(PopulationVector::new(header))
            }
            None => {
                return Err(NexError::UnknownVariableType(
                    header.var_type,
                    header.name.clone(),
                ))
            }
        })
    }

    pub fn header(&self) -> &VariableHeader {
        match self {
            Self::Neuron(v) => &v.header,
            Self::Event(v) => &v.header,
            Self::Interval(v) => &v.header,
            Self::Marker(v) => &v.header,
            Self::Waveform(v) => &v.header,
            Self::Continuous(v) => &v.header,
            Self::PopulationVector(v) => &v.header,
        }
    }

    pub fn header_mut(&mut self) -> &mut VariableHeader {
        match self {
            Self::Neuron(v) => &mut v.header,
            Self::Event(v) => &mut v.header,
            Self::Interval(v) => &mut v.header,
            Self::Marker(v) => &mut v.header,
            Self::Waveform(v) => &mut v.header,
            Self::Continuous(v) => &mut v.header,
            Self::PopulationVector(v) => &mut v.header,
        }
    }

    pub fn metadata(&self) -> &JsonValue {
        match self {
            Self::Neuron(v) => &v.metadata,
            Self::Event(v) => &v.metadata,
            Self::Interval(v) => &v.metadata,
            Self::Marker(v) => &v.metadata,
            Self::Waveform(v) => &v.metadata,
            Self::Continuous(v) => &v.metadata,
            Self::PopulationVector(v) => &v.metadata,
        }
    }

    pub fn metadata_mut(&mut self) -> &mut JsonValue {
        match self {
            Self::Neuron(v) => &mut v.metadata,
            Self::Event(v) => &mut v.metadata,
            Self::Interval(v) => &mut v.metadata,
            Self::Marker(v) => &mut v.metadata,
            Self::Waveform(v) => &mut v.metadata,
            Self::Continuous(v) => &mut v.metadata,
            Self::PopulationVector(v) => &mut v.metadata,
        }
    }

    pub fn name(&self) -> &str {
        &self.header().name
    }

    pub fn var_type(&self) -> i32 {
        self.header().var_type
    }

    pub fn nex_type(&self) -> Option<NexFileVarType> {
        NexFileVarType::from_i32(self.var_type())
    }

    pub fn is_loaded(&self) -> bool {
        match self {
            Self::Event(v) => v.header.count == 0 || !v.timestamps.is_empty(),
            Self::Neuron(v) => v.header.count == 0 || !v.timestamps.is_empty(),
            Self::Interval(v) => v.header.count == 0 || !v.interval_starts.is_empty(),
            Self::Marker(v) => v.header.count == 0 || !v.timestamps.is_empty(),
            Self::Waveform(v) => v.header.count == 0 || !v.timestamps.is_empty(),
            Self::Continuous(v) => v.header.count == 0 || !v.continuous_values.is_empty(),
            Self::PopulationVector(v) => v.header.count == 0 || !v.weights.is_empty(),
        }
    }

    pub fn timestamps(&self) -> Result<Vec<f64>> {
        match self {
            Self::Event(v) => Ok(v.timestamps.as_f64_vec()),
            Self::Neuron(v) => Ok(v.timestamps.as_f64_vec()),
            Self::Marker(v) => Ok(v.timestamps.as_f64_vec()),
            Self::Waveform(v) => Ok(v.timestamps.as_f64_vec()),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "event, neuron, marker, or waveform",
            )),
        }
    }

    pub fn continuous_values(&self) -> Result<&[f64]> {
        match self {
            Self::Continuous(v) => Ok(&v.continuous_values),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "continuous",
            )),
        }
    }

    pub fn intervals(&self) -> Result<(&[f64], &[f64])> {
        match self {
            Self::Interval(v) => Ok((&v.interval_starts, &v.interval_ends)),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "interval",
            )),
        }
    }

    pub fn weights(&self) -> Result<&[f64]> {
        match self {
            Self::PopulationVector(v) => Ok(&v.weights),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "population_vector",
            )),
        }
    }

    pub fn maximum_timestamp(&self) -> f64 {
        match self {
            Self::Neuron(v) => v.maximum_timestamp(),
            Self::Event(v) => v.maximum_timestamp(),
            Self::Interval(v) => v.maximum_timestamp(),
            Self::Marker(v) => v.maximum_timestamp(),
            Self::Waveform(v) => v.maximum_timestamp(),
            Self::Continuous(v) => v.maximum_timestamp(),
            Self::PopulationVector(v) => v.maximum_timestamp(),
        }
    }

    pub fn count_for_header(&self) -> u64 {
        match self {
            Self::Neuron(v) => v.count_for_header(),
            Self::Event(v) => v.count_for_header(),
            Self::Interval(v) => v.count_for_header(),
            Self::Marker(v) => v.count_for_header(),
            Self::Waveform(v) => v.count_for_header(),
            Self::Continuous(v) => v.count_for_header(),
            Self::PopulationVector(v) => v.count_for_header(),
        }
    }

    pub fn bytes_in_data_with_header(&self, hw: &VariableHeader) -> u64 {
        match self {
            Self::Neuron(v) => v.bytes_in_data_with_header(hw),
            Self::Event(v) => v.bytes_in_data_with_header(hw),
            Self::Interval(v) => v.bytes_in_data_with_header(hw),
            Self::Marker(v) => v.bytes_in_data_with_header(hw),
            Self::Waveform(v) => v.bytes_in_data_with_header(hw),
            Self::Continuous(v) => v.bytes_in_data_with_header(hw),
            Self::PopulationVector(v) => v.bytes_in_data_with_header(hw),
        }
    }

    pub fn bytes_in_data(&self) -> u64 {
        self.bytes_in_data_with_header(self.header())
    }

    pub fn calc_marker_length(&mut self) -> Result<()> {
        if let Self::Marker(v) = self {
            v.calc_marker_length_checked()
        } else {
            Ok(())
        }
    }
}

pub(crate) fn max_of_slice_or_zero(values: &[f64]) -> f64 {
    values.iter().copied().fold(0.0, f64::max)
}

pub(crate) fn default_metadata(name: &str) -> JsonValue {
    serde_json::json!({
        "name": name,
        "nameOriginal": name,
    })
}

pub(crate) fn timestamp_payload_bytes(count: usize, hw: &VariableHeader) -> u64 {
    (hw.bytes_in_timestamp() * count) as u64
}
