use crate::compat::round_f64;

#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

use crate::error::{NexError, Result};
use crate::file_data::FileData;
use crate::format::VariableHeader;
use crate::variables::{MarkerVariable, Variable};

/// Prepared write headers for one variable (does not mutate `FileData`).
#[derive(Debug, Clone)]
pub struct VariableWritePlan {
    pub write_header: VariableHeader,
}

pub struct FileWritePlan {
    pub ts_as_64: i32,
    pub variables: Vec<VariableWritePlan>,
}

pub fn prepare_nex5_write_plan(data: &FileData) -> Result<FileWritePlan> {
    let max_ts = data.maximum_timestamp();
    let max_ts_ticks = round_f64(max_ts * data.timestamp_frequency_hz) as i64;
    let ts_as_64 = i32::from(max_ts_ticks > 2_i64.pow(31));
    prepare_write_plan(data, ts_as_64, true)
}

pub fn prepare_nex_write_plan(data: &FileData) -> Result<FileWritePlan> {
    prepare_write_plan(data, 0, false)
}

fn prepare_write_plan(data: &FileData, ts_as_64: i32, is_nex5: bool) -> Result<FileWritePlan> {
    let header_size = if is_nex5 {
        crate::format::NEX5_FILE_HEADER_SIZE
    } else {
        crate::format::NEX_FILE_HEADER_SIZE
    };
    let var_header_size = if is_nex5 {
        crate::format::NEX5_VAR_HEADER_SIZE
    } else {
        crate::format::NEX_VAR_HEADER_SIZE
    };

    let mut data_offset = header_size + data.variables.len() as u64 * var_header_size;
    let mut plans = Vec::with_capacity(data.variables.len());

    for var in &data.variables {
        let mut vh = prepare_variable_write_header(var, ts_as_64, is_nex5)?;
        vh.data_offset = data_offset;
        let bytes = var.bytes_in_data_with_header(&vh);
        data_offset += bytes;
        plans.push(VariableWritePlan { write_header: vh });
    }

    Ok(FileWritePlan {
        ts_as_64,
        variables: plans,
    })
}

fn prepare_variable_write_header(
    var: &Variable,
    ts_as_64: i32,
    is_nex5: bool,
) -> Result<VariableHeader> {
    let mut vh = var.header().clone();
    vh.count = var.count_for_header();

    if is_nex5 {
        vh.ts_data_type = ts_as_64;
        vh.version = 500;
        vh.cont_frag_index_type = 0;
    } else {
        vh.ts_data_type = 0;
        vh.version = 102;
        vh.cont_data_type = 0;
        vh.cont_frag_index_type = 0;
        vh.marker_data_type = 0;
    }

    if let Variable::Marker(marker) = var {
        let params = marker_write_params(marker)?;
        vh.n_markers = params.n_markers;
        vh.marker_length = params.marker_length;
        vh.marker_data_type = if is_nex5 { params.marker_data_type } else { 0 };
    }

    if matches!(var, Variable::Continuous(_) | Variable::Waveform(_)) {
        let cont_type = if is_nex5 { vh.cont_data_type } else { 0 };
        let scaling = cont_scaling(var, cont_type);
        vh.ad_to_mv = scaling.0;
        vh.mv_offset = scaling.1;
    }

    Ok(vh)
}

struct MarkerWriteParams {
    n_markers: i32,
    marker_length: i32,
    marker_data_type: i32,
}

fn marker_write_params(marker: &MarkerVariable) -> Result<MarkerWriteParams> {
    let n_markers = marker.marker_field_names.len() as i32;
    if marker.all_marker_values_are_numeric() {
        Ok(MarkerWriteParams {
            n_markers,
            marker_length: 6,
            marker_data_type: 1,
        })
    } else {
        let mut max_string_length = 0usize;
        for field in &marker.marker_fields {
            for value in field {
                match value {
                    crate::variables::MarkerFieldValue::String(s) => {
                        max_string_length = max_string_length.max(s.len());
                    }
                    crate::variables::MarkerFieldValue::Number(_) => {
                        return Err(NexError::InvalidMarkerValues);
                    }
                }
            }
        }
        Ok(MarkerWriteParams {
            n_markers,
            marker_length: (max_string_length + 1).max(6) as i32,
            marker_data_type: 0,
        })
    }
}

fn cont_scaling(var: &Variable, cont_data_type: i32) -> (f64, f64) {
    match var {
        Variable::Continuous(v) => v.cont_scaling(cont_data_type),
        Variable::Waveform(v) => v.cont_scaling(cont_data_type),
        _ => (1.0, 0.0),
    }
}

pub fn has_writable_payload(data: &FileData) -> bool {
    !data.variables.is_empty() && data.variables.iter().any(|v| v.count_for_header() > 0)
}

pub fn ensure_all_payloads_loaded(data: &FileData) -> Result<()> {
    for var in &data.variables {
        if var.header().count > 0 && !var.is_loaded() {
            return Err(NexError::VariableNotLoaded(var.name().to_string()));
        }
    }
    Ok(())
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::FileData;

    #[test]
    fn nex_full_header_section_bytes() {
        let mut data = FileData::new(100_000.0, "").unwrap();
        data.add_event("ev", vec![1.0, 2.0]).unwrap();
        data.add_neuron("nr", vec![0.5], 1, 1, 0.0, 0.0).unwrap();
        let plan = prepare_nex_write_plan(&data).unwrap();
        let mut buf = Vec::new();
        let fh = crate::format::FileHeader {
            magic_number: crate::format::NEX_MAGIC,
            nex_file_version: 106,
            comment: String::new(),
            frequency: data.timestamp_frequency_hz,
            beg_ticks: 0,
            end_ticks: 1,
            num_vars: 2,
            ..Default::default()
        };
        fh.write_to_nex(&mut buf).unwrap();
        for entry in &plan.variables {
            entry.write_header.write_to_nex(&mut buf).unwrap();
        }
        assert_eq!(buf.len(), 960);
        assert_eq!(
            i32::from_le_bytes(buf[544 + 208 + 4..544 + 208 + 8].try_into().unwrap()),
            102
        );
    }
}
