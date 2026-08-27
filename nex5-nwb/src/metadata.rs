//! Encode/decode nex5-specific metadata in NWB session fields.

use serde::{Deserialize, Serialize};

const NEURON_NAMES_TAG: &str = "\x1enex5_neurons=";
const INTERVAL_TAG: &str = "nex5_interval:";
const MARKER_TAG: &str = "nex5_marker:";
const WAVEFORM_TAG: &str = "nex5_waveform:";
const JSON_TAG: &str = "\x1enex5_json=";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionPayload {
    pub neuron_names: Vec<String>,
    pub markers: Vec<MarkerPayload>,
    pub waveforms: Vec<WaveformPayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MarkerPayload {
    pub name: String,
    pub field_names: Vec<String>,
    /// Indices into `field_names` for columns stored as strings in JSON.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub string_field_indices: Vec<usize>,
    /// String marker values `[string_field][marker_index]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub string_fields: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WaveformPayload {
    pub name: String,
    pub n_points: u64,
    pub sampling_rate: f64,
    pub pre_thr_time: f64,
    pub cont_data_type: i32,
}

pub fn encode_session_description(base: &str, payload: &SessionPayload) -> String {
    let mut out = encode_neuron_names(base, &payload.neuron_names);
    if !(payload.markers.is_empty() && payload.waveforms.is_empty()) {
        if let Ok(json) = serde_json::to_string(payload) {
            out.push_str(JSON_TAG);
            out.push_str(&json);
        }
    }
    out
}

pub fn decode_session_description(description: &str) -> (String, SessionPayload) {
    let (base, json_part) = if let Some((b, j)) = description.split_once(JSON_TAG) {
        (b, Some(j))
    } else {
        (description, None)
    };
    let (comment, neuron_names) = decode_neuron_names(base);
    if let Some(j) = json_part {
        if let Ok(payload) = serde_json::from_str::<SessionPayload>(j) {
            return (comment, payload);
        }
    }
    (
        comment,
        SessionPayload {
            neuron_names,
            ..Default::default()
        },
    )
}

pub fn encode_neuron_names(description: &str, names: &[String]) -> String {
    if names.is_empty() {
        return description.to_string();
    }
    let payload = names.join("\x1f");
    format!("{description}{NEURON_NAMES_TAG}{payload}")
}

pub fn decode_neuron_names(description: &str) -> (String, Vec<String>) {
    let base = description.split(JSON_TAG).next().unwrap_or(description);
    let Some((base, names_part)) = base.split_once(NEURON_NAMES_TAG) else {
        return (base.to_string(), Vec::new());
    };
    let names = names_part
        .split('\x1f')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    (base.to_string(), names)
}

pub fn interval_start_name(base: &str) -> String {
    format!("{INTERVAL_TAG}{base}__start")
}

pub fn interval_end_name(base: &str) -> String {
    format!("{INTERVAL_TAG}{base}__end")
}

pub fn parse_interval_name(path: &str) -> Option<(String, bool)> {
    let short = path.rsplit('/').next().unwrap_or(path);
    let rest = short.strip_prefix(INTERVAL_TAG)?;
    if let Some(base) = rest.strip_suffix("__start") {
        Some((base.to_string(), true))
    } else {
        rest.strip_suffix("__end")
            .map(|base| (base.to_string(), false))
    }
}

pub fn marker_ts_name(base: &str) -> String {
    format!("{MARKER_TAG}{base}__ts")
}

pub fn marker_field_name(base: &str, field_index: usize) -> String {
    format!("{MARKER_TAG}{base}__f{field_index}")
}

pub fn parse_marker_path(path: &str) -> Option<MarkerPath> {
    let short = path.rsplit('/').next().unwrap_or(path);
    let rest = short.strip_prefix(MARKER_TAG)?;
    if let Some(base) = rest.strip_suffix("__ts") {
        return Some(MarkerPath::Timestamps(base.to_string()));
    }
    if let Some((base, idx_str)) = rest.rsplit_once("__f") {
        if let Ok(field_index) = idx_str.parse::<usize>() {
            return Some(MarkerPath::Field {
                name: base.to_string(),
                field_index,
            });
        }
    }
    None
}

pub enum MarkerPath {
    Timestamps(String),
    Field { name: String, field_index: usize },
}

pub fn waveform_ts_name(base: &str) -> String {
    format!("{WAVEFORM_TAG}{base}__ts")
}

pub fn waveform_samples_name(base: &str) -> String {
    format!("{WAVEFORM_TAG}{base}__samples")
}

pub fn parse_waveform_path(path: &str) -> Option<WaveformPath> {
    let short = path.rsplit('/').next().unwrap_or(path);
    let rest = short.strip_prefix(WAVEFORM_TAG)?;
    if let Some(base) = rest.strip_suffix("__ts") {
        return Some(WaveformPath::Timestamps(base.to_string()));
    }
    if let Some(base) = rest.strip_suffix("__samples") {
        return Some(WaveformPath::Samples(base.to_string()));
    }
    None
}

pub enum WaveformPath {
    Timestamps(String),
    Samples(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_payload_roundtrip() {
        let payload = SessionPayload {
            neuron_names: vec!["u1".into()],
            markers: vec![MarkerPayload {
                name: "m1".into(),
                field_names: vec!["code".into()],
                string_field_indices: vec![0],
                string_fields: vec![vec!["a".into(), "b".into()]],
            }],
            waveforms: vec![WaveformPayload {
                name: "wf".into(),
                n_points: 32,
                sampling_rate: 40_000.0,
                pre_thr_time: 0.001,
                cont_data_type: 1,
            }],
        };
        let encoded = encode_session_description("session", &payload);
        let (desc, decoded) = decode_session_description(&encoded);
        assert_eq!(desc, "session");
        assert_eq!(decoded.markers, payload.markers);
        assert_eq!(decoded.waveforms, payload.waveforms);
    }
}
