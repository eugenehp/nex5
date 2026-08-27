use consus_nwb::file::{NwbFile, NwbFileBuilder};
use consus_nwb::model::TimeSeries;
use consus_nwb::UnitsTable;
use nex5file::MarkerFieldValue;
use nex5file::{FileData, Variable};
use std::collections::BTreeMap;

use crate::error::{from_consus, Result, NwbError};
use crate::metadata::{
    decode_session_description, encode_session_description, interval_end_name, interval_start_name,
    marker_field_name, marker_ts_name, parse_interval_name, parse_marker_path, parse_waveform_path,
    waveform_samples_name, waveform_ts_name, MarkerPath, MarkerPayload, SessionPayload,
    WaveformPath, WaveformPayload,
};
use crate::options::{NwbReadOptions, NwbWriteOptions};

const DEFAULT_SESSION_START: &str = "1970-01-01T00:00:00Z";
const REQUIRED_GROUPS: &[&str] = &[
    "acquisition",
    "analysis",
    "processing",
    "stimulus",
    "general",
];

type IntervalPartsMap = BTreeMap<String, (Option<Vec<f64>>, Option<Vec<f64>>)>;
type MarkerFieldParts = BTreeMap<(String, usize), Vec<f64>>;
type WaveformParts = BTreeMap<String, (Option<Vec<f64>>, Option<Vec<f64>>)>;

struct NeuronExport {
    names: Vec<String>,
    spike_trains: Vec<Vec<f64>>,
}

/// Convert in-memory nex5 data to NWB HDF5 bytes.
pub fn file_data_to_nwb_bytes(data: &FileData, options: &NwbWriteOptions) -> Result<Vec<u8>> {
    let neurons = collect_neurons(data);
    let payload = build_session_payload(data, &neurons.names);
    let identifier = options
        .identifier
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if data.comment.is_empty() {
                "nex5-session".to_string()
            } else {
                data.comment.clone()
            }
        });
    let base_description = options
        .session_description
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            if data.comment.is_empty() {
                "Converted from nex5file".to_string()
            } else {
                data.comment.clone()
            }
        });
    let session_description = if options.preserve_neuron_names {
        encode_session_description(&base_description, &payload)
    } else {
        base_description
    };
    let session_start_time = options
        .session_start_time
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_SESSION_START.to_string());

    let mut builder = NwbFileBuilder::new(
        &options.nwb_version,
        identifier,
        session_description,
        session_start_time,
    )
    .map_err(from_consus)?;

    for group in REQUIRED_GROUPS {
        builder.write_empty_group(group).map_err(from_consus)?;
    }

    if !neurons.spike_trains.is_empty() {
        let ids: Vec<u64> = (0..neurons.spike_trains.len() as u64).collect();
        let table = UnitsTable::from_parts(neurons.spike_trains, Some(ids)).map_err(from_consus)?;
        builder.write_units_table(&table).map_err(from_consus)?;
    }

    for var in &data.variables {
        match var {
            Variable::Event(ev) => {
                write_event_series(&mut builder, &ev.header.name, &ev.timestamps.as_f64_vec())?;
            }
            Variable::Continuous(cont) => {
                if cont.continuous_values.is_empty() {
                    continue;
                }
                let path = sanitize_hdf5_name(cont.header.name.as_str());
                let start = cont
                    .fragment_timestamps
                    .first()
                    .copied()
                    .unwrap_or(data.beg_seconds);
                let rate = cont.header.sampling_rate;
                if rate <= 0.0 {
                    return Err(NwbError::Other(format!(
                        "continuous variable '{}' has invalid sampling rate",
                        cont.header.name
                    )));
                }
                let ts = TimeSeries::with_rate(
                    path,
                    cont.continuous_values.clone(),
                    start,
                    rate,
                );
                builder.write_time_series(&ts).map_err(from_consus)?;
            }
            Variable::Interval(intv) => {
                write_event_series(
                    &mut builder,
                    &interval_start_name(&intv.header.name),
                    &intv.interval_starts,
                )?;
                write_event_series(
                    &mut builder,
                    &interval_end_name(&intv.header.name),
                    &intv.interval_ends,
                )?;
            }
            Variable::Marker(m) => {
                write_event_series(
                    &mut builder,
                    &marker_ts_name(&m.header.name),
                    &m.timestamps.as_f64_vec(),
                )?;
                for (fi, field) in m.marker_fields.iter().enumerate() {
                    if field.iter().all(|v| matches!(v, MarkerFieldValue::Number(_))) {
                        let values: Vec<f64> = field
                            .iter()
                            .map(|v| match v {
                                MarkerFieldValue::Number(n) => f64::from(*n),
                                MarkerFieldValue::String(_) => 0.0,
                            })
                            .collect();
                        write_value_series(
                            &mut builder,
                            &marker_field_name(&m.header.name, fi),
                            &m.timestamps.as_f64_vec(),
                            &values,
                        )?;
                    }
                }
            }
            Variable::Waveform(w) => {
                write_event_series(
                    &mut builder,
                    &waveform_ts_name(&w.header.name),
                    &w.timestamps.as_f64_vec(),
                )?;
                let flat: Vec<f64> = w
                    .waveform_values
                    .iter()
                    .map(|&v| f64::from(v))
                    .collect();
                let path = sanitize_hdf5_name(&waveform_samples_name(&w.header.name));
                let rate = w.header.sampling_rate.max(1.0);
                let ts = TimeSeries::with_rate(path, flat, 0.0, rate);
                builder.write_time_series(&ts).map_err(from_consus)?;
            }
            Variable::Neuron(_) => {}
            _ => {}
        }
    }

    builder.finish().map_err(from_consus)
}

fn build_session_payload(data: &FileData, neuron_names: &[String]) -> SessionPayload {
    let mut markers = Vec::new();
    let mut waveforms = Vec::new();
    for var in &data.variables {
        match var {
            Variable::Marker(m) => {
                let mut string_field_indices = Vec::new();
                let mut string_fields = Vec::new();
                for (fi, field) in m.marker_fields.iter().enumerate() {
                    if field
                        .iter()
                        .any(|v| matches!(v, MarkerFieldValue::String(_)))
                    {
                        string_field_indices.push(fi);
                        let col: Vec<String> = field
                            .iter()
                            .map(|v| match v {
                                MarkerFieldValue::Number(n) => n.to_string(),
                                MarkerFieldValue::String(s) => s.clone(),
                            })
                            .collect();
                        string_fields.push(col);
                    }
                }
                markers.push(MarkerPayload {
                    name: m.header.name.clone(),
                    field_names: m.marker_field_names.clone(),
                    string_field_indices,
                    string_fields,
                });
            }
            Variable::Waveform(w) => {
                waveforms.push(WaveformPayload {
                    name: w.header.name.clone(),
                    n_points: w.header.n_points_wave,
                    sampling_rate: w.header.sampling_rate,
                    pre_thr_time: w.header.pre_thr_time,
                    cont_data_type: w.header.cont_data_type,
                });
            }
            _ => {}
        }
    }
    SessionPayload {
        neuron_names: neuron_names.to_vec(),
        markers,
        waveforms,
    }
}

fn write_event_series(
    builder: &mut NwbFileBuilder,
    name: &str,
    timestamps: &[f64],
) -> Result<()> {
    if timestamps.is_empty() {
        return Ok(());
    }
    let path = sanitize_hdf5_name(name);
    let data_values = vec![1.0; timestamps.len()];
    let ts = TimeSeries::with_timestamps(path, data_values, timestamps.to_vec());
    builder.write_time_series(&ts).map_err(from_consus)?;
    Ok(())
}

fn write_value_series(
    builder: &mut NwbFileBuilder,
    name: &str,
    timestamps: &[f64],
    values: &[f64],
) -> Result<()> {
    if timestamps.is_empty() || timestamps.len() != values.len() {
        return Ok(());
    }
    let path = sanitize_hdf5_name(name);
    let ts = TimeSeries::with_timestamps(path, values.to_vec(), timestamps.to_vec());
    builder.write_time_series(&ts).map_err(from_consus)?;
    Ok(())
}

/// Convert NWB HDF5 bytes to in-memory nex5 data.
#[allow(clippy::type_complexity)]
pub fn nwb_bytes_to_file_data(bytes: &[u8], options: &NwbReadOptions) -> Result<FileData> {
    if options.timestamp_frequency_hz <= 0.0 {
        return Err(NwbError::Other(
            "timestamp_frequency_hz must be positive".to_string(),
        ));
    }

    let nwb = NwbFile::open(bytes).map_err(from_consus)?;
    let meta = nwb.session_metadata().map_err(from_consus)?;
    let (comment, session_payload) = decode_session_description(meta.session_description());
    let mut data = FileData::new(options.timestamp_frequency_hz, &comment)?;
    data.comment = comment;
    data.beg_seconds = 0.0;

    if let Ok(table) = nwb.units_table() {
        ingest_units_table(&mut data, &table, &session_payload.neuron_names)?;
    }

    let mut ts_paths = nwb.list_acquisition().unwrap_or_default();
    if let Ok(root_paths) = nwb.list_time_series("") {
        for path in root_paths {
            if !ts_paths.iter().any(|p| p == &path) {
                ts_paths.push(path);
            }
        }
    }

    let mut interval_parts: IntervalPartsMap = BTreeMap::new();
    let mut marker_ts: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut marker_fields: MarkerFieldParts = BTreeMap::new();
    let mut waveform_parts: WaveformParts = BTreeMap::new();

    for path in ts_paths {
        if let Some((base, is_start)) = parse_interval_name(&path) {
            let ts = nwb.time_series(&path).map_err(from_consus)?;
            let stamps = ts.timestamps().unwrap_or(&[]).to_vec();
            let entry = interval_parts.entry(base).or_insert((None, None));
            if is_start {
                entry.0 = Some(stamps);
            } else {
                entry.1 = Some(stamps);
            }
            continue;
        }

        if let Some(marker_path) = parse_marker_path(&path) {
            let ts = nwb.time_series(&path).map_err(from_consus)?;
            match marker_path {
                MarkerPath::Timestamps(name) => {
                    if let Some(stamps) = ts.timestamps() {
                        marker_ts.insert(name, stamps.to_vec());
                    }
                }
                MarkerPath::Field { name, field_index } => {
                    marker_fields.insert((name, field_index), ts.data().to_vec());
                }
            }
            continue;
        }

        if let Some(wf_path) = parse_waveform_path(&path) {
            let ts = nwb.time_series(&path).map_err(from_consus)?;
            match wf_path {
                WaveformPath::Timestamps(name) => {
                    let entry = waveform_parts.entry(name).or_insert((None, None));
                    entry.0 = ts.timestamps().map(|t| t.to_vec());
                }
                WaveformPath::Samples(name) => {
                    let entry = waveform_parts.entry(name).or_insert((None, None));
                    entry.1 = Some(ts.data().to_vec());
                }
            }
            continue;
        }

        let ts = nwb.time_series(&path).map_err(from_consus)?;
        let short_name = path
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(path.as_str())
            .to_string();

        if ts.has_rate() {
            let rate = ts.rate().unwrap_or(0.0);
            let start = ts.starting_time().unwrap_or(0.0);
            data.add_cont_var_with_floats_single_fragment(
                short_name,
                rate,
                start,
                ts.data().to_vec(),
            )?;
        } else if let Some(timestamps) = ts.timestamps() {
            if looks_like_event(ts.data()) {
                data.add_event(short_name, timestamps.to_vec())?;
            } else if timestamps.len() == ts.data().len() && !timestamps.is_empty() {
                let rate = estimate_sampling_rate(timestamps);
                data.add_cont_var_with_floats_all_timestamps(
                    short_name,
                    rate,
                    timestamps.to_vec(),
                    ts.data().to_vec(),
                )?;
            }
        }
    }

    for (name, (starts, ends)) in interval_parts {
        if let (Some(starts), Some(ends)) = (starts, ends) {
            let pairs: Vec<(f64, f64)> = starts.into_iter().zip(ends).collect();
            data.add_interval_as_pairs_start_end(name, &pairs)?;
        }
    }

    for marker_meta in &session_payload.markers {
        let timestamps = marker_ts
            .get(&marker_meta.name)
            .cloned()
            .unwrap_or_default();
        if timestamps.is_empty() {
            continue;
        }
        let mut fields: Vec<Vec<MarkerFieldValue>> =
            Vec::with_capacity(marker_meta.field_names.len());
        for fi in 0..marker_meta.field_names.len() {
            if marker_meta
                .string_field_indices
                .iter()
                .position(|&idx| idx == fi)
                .and_then(|pos| marker_meta.string_fields.get(pos))
                .is_some_and(|col| !col.is_empty())
            {
                let pos = marker_meta
                    .string_field_indices
                    .iter()
                    .position(|&idx| idx == fi)
                    .unwrap();
                let col: Vec<MarkerFieldValue> = marker_meta.string_fields[pos]
                    .iter()
                    .map(|s| MarkerFieldValue::String(s.clone()))
                    .collect();
                fields.push(col);
            } else if let Some(nums) = marker_fields.get(&(marker_meta.name.clone(), fi)) {
                let col: Vec<MarkerFieldValue> = nums
                    .iter()
                    .map(|&n| MarkerFieldValue::Number(n as u32))
                    .collect();
                fields.push(col);
            } else {
                fields.push(vec![MarkerFieldValue::Number(0); timestamps.len()]);
            }
        }
        data.add_marker(
            &marker_meta.name,
            timestamps,
            marker_meta.field_names.clone(),
            fields,
        )?;
    }

    for wf_meta in &session_payload.waveforms {
        if let Some((timestamps, samples)) = waveform_parts.get(&wf_meta.name) {
            let ts = timestamps.clone().unwrap_or_default();
            let flat = samples.clone().unwrap_or_default();
            if ts.is_empty() || flat.is_empty() {
                continue;
            }
            let n_points = wf_meta.n_points as usize;
            if n_points == 0 || flat.len() % n_points != 0 {
                continue;
            }
            let nested: Vec<Vec<f32>> = flat
                .chunks(n_points)
                .map(|chunk| chunk.iter().map(|&v| v as f32).collect())
                .collect();
            data.add_wave_var_with_floats(
                &wf_meta.name,
                wf_meta.sampling_rate,
                ts,
                nested,
            )?;
        }
    }

    data.end_seconds = data
        .variables
        .iter()
        .map(|v| v.maximum_timestamp())
        .fold(data.beg_seconds, f64::max);
    Ok(data)
}

fn collect_neurons(data: &FileData) -> NeuronExport {
    let mut names = Vec::new();
    let mut spike_trains = Vec::new();
    for var in &data.variables {
        if let Variable::Neuron(nr) = var {
            names.push(nr.header.name.clone());
            spike_trains.push(nr.timestamps.as_f64_vec());
        }
    }
    NeuronExport { names, spike_trains }
}

fn ingest_units_table(
    data: &mut FileData,
    table: &UnitsTable,
    encoded_names: &[String],
) -> Result<()> {
    for (idx, spikes) in table.spike_times_per_unit().iter().enumerate() {
        let name = encoded_names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("unit_{idx}"));
        data.add_neuron(name, spikes.clone(), 0, idx as i32, 0.0, 0.0)?;
    }
    Ok(())
}

fn sanitize_hdf5_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.replace('/', "_")
    }
}

fn looks_like_event(data: &[f64]) -> bool {
    data.is_empty() || data.iter().all(|&v| (v - 1.0).abs() < 1e-9)
}

fn estimate_sampling_rate(timestamps: &[f64]) -> f64 {
    if timestamps.len() < 2 {
        return 1.0;
    }
    let dt = timestamps[1] - timestamps[0];
    if dt > 0.0 {
        1.0 / dt
    } else {
        1.0
    }
}
