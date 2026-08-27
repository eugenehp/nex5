/// Options when reading an NWB file into [`nex5file::FileData`].
#[derive(Debug, Clone)]
pub struct NwbReadOptions {
    /// Timestamp frequency stored on the resulting `FileData` (NWB has no direct equivalent).
    pub timestamp_frequency_hz: f64,
}

impl Default for NwbReadOptions {
    fn default() -> Self {
        Self {
            timestamp_frequency_hz: 100_000.0,
        }
    }
}

/// Options when writing [`nex5file::FileData`] to NWB.
#[derive(Debug, Clone)]
pub struct NwbWriteOptions {
    /// NWB schema version string (e.g. `"2.7.0"`).
    pub nwb_version: String,
    /// Session identifier written to the root `identifier` attribute.
    pub identifier: Option<String>,
    /// Root `session_description` attribute.
    pub session_description: Option<String>,
    /// ISO-8601 `session_start_time` (defaults to a fixed epoch when unset).
    pub session_start_time: Option<String>,
    /// Embed neuron variable names in `session_description` for round-trip.
    pub preserve_neuron_names: bool,
}

impl Default for NwbWriteOptions {
    fn default() -> Self {
        Self {
            nwb_version: "2.7.0".to_string(),
            identifier: None,
            session_description: None,
            session_start_time: None,
            preserve_neuron_names: true,
        }
    }
}
