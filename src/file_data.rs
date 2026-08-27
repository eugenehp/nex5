use crate::compat::{vec, HashMap, String, ToString, Vec};
use crate::error::{NexError, Result};
use crate::format::VariableHeader;
use crate::variables::{
    ContinuousVariable, EventVariable, IntervalVariable, MarkerFieldValue, MarkerVariable,
    NeuronVariable, NexFileVarType, PopulationVector, Timestamps, Variable, WaveformVariable,
};
use core::ops::{Index, IndexMut};
use serde_json::Value as JsonValue;

/// In-memory representation of a .nex or .nex5 file, mirroring the Python `FileData` class.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct FileData {
    pub timestamp_frequency_hz: f64,
    pub comment: String,
    pub beg_seconds: f64,
    pub end_seconds: f64,
    pub metadata: JsonValue,
    pub variables: Vec<Variable>,
    #[serde(skip)]
    name_index: HashMap<String, usize>,
}

impl<'de> serde::Deserialize<'de> for FileData {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            timestamp_frequency_hz: f64,
            comment: String,
            beg_seconds: f64,
            end_seconds: f64,
            metadata: JsonValue,
            variables: Vec<Variable>,
        }
        let h = Helper::deserialize(deserializer)?;
        let mut data = Self {
            timestamp_frequency_hz: h.timestamp_frequency_hz,
            comment: h.comment,
            beg_seconds: h.beg_seconds,
            end_seconds: h.end_seconds,
            metadata: h.metadata,
            variables: h.variables,
            name_index: HashMap::new(),
        };
        data.rebuild_index();
        Ok(data)
    }
}

impl FileData {
    pub fn new(ts_frequency: f64, comment: impl Into<String>) -> Result<Self> {
        if ts_frequency <= 0.0 {
            return Err(NexError::InvalidTimestampFrequency);
        }
        Ok(Self::from_parts(
            ts_frequency,
            comment.into(),
            0.0,
            0.0,
            JsonValue::Object(Default::default()),
            Vec::new(),
        ))
    }

    pub(crate) fn from_header(file_header: &crate::format::FileHeader) -> Self {
        Self::from_parts(
            file_header.frequency,
            file_header.comment.clone(),
            file_header.beg_seconds,
            file_header.end_seconds,
            JsonValue::Object(Default::default()),
            Vec::new(),
        )
    }

    fn from_parts(
        timestamp_frequency_hz: f64,
        comment: String,
        beg_seconds: f64,
        end_seconds: f64,
        metadata: JsonValue,
        variables: Vec<Variable>,
    ) -> Self {
        let mut data = Self {
            timestamp_frequency_hz,
            comment,
            beg_seconds,
            end_seconds,
            metadata,
            variables,
            name_index: HashMap::new(),
        };
        data.rebuild_index();
        data
    }

    pub(crate) fn rebuild_index(&mut self) {
        self.name_index = self
            .variables
            .iter()
            .enumerate()
            .map(|(i, v)| (v.name().to_string(), i))
            .collect();
    }

    pub(crate) fn index_of(&self, name: &str) -> Result<usize> {
        self.name_index
            .get(name)
            .copied()
            .ok_or_else(|| NexError::VariableNotFound(name.to_string()))
    }

    pub fn get_variable(&self, name: &str) -> Result<&Variable> {
        Ok(&self.variables[self.index_of(name)?])
    }

    pub fn get_variable_mut(&mut self, name: &str) -> Result<&mut Variable> {
        let idx = self.index_of(name)?;
        Ok(&mut self.variables[idx])
    }

    pub fn event(&self, name: &str) -> Result<&EventVariable> {
        match self.get_variable(name)? {
            Variable::Event(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "event",
            )),
        }
    }

    pub fn neuron(&self, name: &str) -> Result<&NeuronVariable> {
        match self.get_variable(name)? {
            Variable::Neuron(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "neuron",
            )),
        }
    }

    pub fn interval(&self, name: &str) -> Result<&IntervalVariable> {
        match self.get_variable(name)? {
            Variable::Interval(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "interval",
            )),
        }
    }

    pub fn marker(&self, name: &str) -> Result<&MarkerVariable> {
        match self.get_variable(name)? {
            Variable::Marker(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "marker",
            )),
        }
    }

    pub fn waveform(&self, name: &str) -> Result<&WaveformVariable> {
        match self.get_variable(name)? {
            Variable::Waveform(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "waveform",
            )),
        }
    }

    pub fn continuous(&self, name: &str) -> Result<&ContinuousVariable> {
        match self.get_variable(name)? {
            Variable::Continuous(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "continuous",
            )),
        }
    }

    pub fn population_vector(&self, name: &str) -> Result<&PopulationVector> {
        match self.get_variable(name)? {
            Variable::PopulationVector(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "population_vector",
            )),
        }
    }

    pub fn event_mut(&mut self, name: &str) -> Result<&mut EventVariable> {
        match self.get_variable_mut(name)? {
            Variable::Event(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "event",
            )),
        }
    }

    pub fn neuron_mut(&mut self, name: &str) -> Result<&mut NeuronVariable> {
        match self.get_variable_mut(name)? {
            Variable::Neuron(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "neuron",
            )),
        }
    }

    pub fn continuous_mut(&mut self, name: &str) -> Result<&mut ContinuousVariable> {
        match self.get_variable_mut(name)? {
            Variable::Continuous(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "continuous",
            )),
        }
    }

    pub fn waveform_mut(&mut self, name: &str) -> Result<&mut WaveformVariable> {
        match self.get_variable_mut(name)? {
            Variable::Waveform(v) => Ok(v),
            other => Err(NexError::WrongVariableType(
                other.name().to_string(),
                "waveform",
            )),
        }
    }

    pub fn is_variable_loaded(&self, name: &str) -> Result<bool> {
        Ok(self.get_variable(name)?.is_loaded())
    }

    pub fn delete_variable(&mut self, name: &str) -> Result<()> {
        let pos = self.index_of(name)?;
        self.variables.remove(pos);
        self.rebuild_index();
        Ok(())
    }

    pub fn doc_comment(&self) -> &str {
        &self.comment
    }

    pub fn timestamp_frequency(&self) -> f64 {
        self.timestamp_frequency_hz
    }

    pub fn doc_start_time(&self) -> f64 {
        self.beg_seconds
    }

    pub fn doc_end_time(&self) -> f64 {
        self.end_seconds
    }

    fn var_names(&self, var_type: NexFileVarType) -> Vec<String> {
        self.variables
            .iter()
            .filter(|v| v.var_type() == var_type as i32)
            .map(|v| v.name().to_string())
            .collect()
    }

    pub fn neuron_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Neuron)
    }

    pub fn event_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Event)
    }

    pub fn interval_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Interval)
    }

    pub fn wave_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Waveform)
    }

    pub fn marker_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Marker)
    }

    pub fn continuous_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::Continuous)
    }

    pub fn pop_vector_names(&self) -> Vec<String> {
        self.var_names(NexFileVarType::PopulationVector)
    }

    /// All variable names in file order.
    pub fn variable_names(&self) -> Vec<String> {
        self.variables.iter().map(|v| v.name().to_string()).collect()
    }

    pub(crate) fn maximum_timestamp(&self) -> f64 {
        self.variables
            .iter()
            .map(|v| v.maximum_timestamp())
            .fold(0.0, f64::max)
    }

    fn add_variable(&mut self, var: Variable) -> Result<()> {
        if var.var_type() < 0 || var.name().is_empty() {
            return Err(NexError::InvalidVariable);
        }
        let name = var.name().to_string();
        if self.name_index.contains_key(&name) {
            return Err(NexError::DuplicateVariable(name));
        }
        let idx = self.variables.len();
        self.variables.push(var);
        self.name_index.insert(name, idx);
        self.end_seconds = self.maximum_timestamp();
        Ok(())
    }

    /// Rename a variable (updates the name index).
    pub fn rename_variable(&mut self, old_name: &str, new_name: impl Into<String>) -> Result<()> {
        let new_name = new_name.into();
        if new_name.is_empty() {
            return Err(NexError::InvalidVariable);
        }
        if self.name_index.contains_key(&new_name) {
            return Err(NexError::DuplicateVariable(new_name));
        }
        let idx = self.index_of(old_name)?;
        self.variables[idx].header_mut().name = new_name.clone();
        self.name_index.remove(old_name);
        self.name_index.insert(new_name, idx);
        Ok(())
    }

    /// Shallow copy with only the listed variables (headers + loaded payloads).
    pub fn subset(&self, names: &[impl AsRef<str>]) -> Result<Self> {
        let mut out = Self::new(self.timestamp_frequency_hz, &self.comment)?;
        out.beg_seconds = self.beg_seconds;
        out.end_seconds = self.end_seconds;
        out.metadata = self.metadata.clone();
        for name in names {
            let name = name.as_ref();
            let var = self.get_variable(name)?.clone();
            out.add_variable(var)?;
        }
        Ok(out)
    }

    /// Append variables from `other` (frequencies must match).
    pub fn merge(&mut self, other: &FileData) -> Result<()> {
        if (self.timestamp_frequency_hz - other.timestamp_frequency_hz).abs() > f64::EPSILON {
            return Err(NexError::InvalidTimestampFrequency);
        }
        if self.comment.is_empty() && !other.comment.is_empty() {
            self.comment = other.comment.clone();
        }
        self.beg_seconds = self.beg_seconds.min(other.beg_seconds);
        for var in &other.variables {
            self.add_variable(var.clone())?;
        }
        self.end_seconds = self.maximum_timestamp();
        Ok(())
    }

    pub fn add_event(&mut self, ev_name: impl Into<String>, ev_timestamps: Vec<f64>) -> Result<()> {
        let name = ev_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Event as i32,
            name,
            ..Default::default()
        };
        let mut ev = EventVariable::new(header);
        ev.timestamps = Timestamps::from(ev_timestamps);
        self.add_variable(Variable::Event(ev))
    }

    pub fn add_neuron(
        &mut self,
        nr_name: impl Into<String>,
        nr_timestamps: Vec<f64>,
        wire: i32,
        unit: i32,
        x_position: f64,
        y_position: f64,
    ) -> Result<()> {
        let name = nr_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Neuron as i32,
            name,
            wire,
            unit,
            x_pos: x_position,
            y_pos: y_position,
            ..Default::default()
        };
        let mut nr = NeuronVariable::new(header);
        nr.timestamps = Timestamps::from(nr_timestamps);
        self.add_variable(Variable::Neuron(nr))
    }

    pub fn add_interval_as_pairs_start_end(
        &mut self,
        int_name: impl Into<String>,
        intervals_as_pairs: &[(f64, f64)],
    ) -> Result<()> {
        let name = int_name.into();
        let (starts, ends): (Vec<f64>, Vec<f64>) = intervals_as_pairs.iter().copied().unzip();
        crate::validation::validate_intervals(&starts, &ends)?;
        let header = VariableHeader {
            var_type: NexFileVarType::Interval as i32,
            name,
            ..Default::default()
        };
        let mut int_var = IntervalVariable::new(header);
        int_var.interval_starts = starts;
        int_var.interval_ends = ends;
        self.add_variable(Variable::Interval(int_var))
    }

    pub fn add_marker(
        &mut self,
        marker_name: impl Into<String>,
        timestamps: Vec<f64>,
        field_names: Vec<String>,
        fields: Vec<Vec<MarkerFieldValue>>,
    ) -> Result<()> {
        if field_names.len() != fields.len() {
            return Err(NexError::InvalidMarkerParameters);
        }
        for f in &fields {
            if f.len() != timestamps.len() {
                return Err(NexError::InvalidMarkerParameters);
            }
        }

        let name = marker_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Marker as i32,
            name,
            n_markers: field_names.len() as i32,
            ..Default::default()
        };
        let mut marker_var = MarkerVariable::new(header);
        marker_var.timestamps = Timestamps::from(timestamps);
        marker_var.marker_field_names = field_names;
        marker_var.marker_fields = fields;
        self.add_variable(Variable::Marker(marker_var))
    }

    pub fn add_population_vector(
        &mut self,
        name: impl Into<String>,
        weights: Vec<f64>,
    ) -> Result<()> {
        let name = name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::PopulationVector as i32,
            name,
            ..Default::default()
        };
        let mut pv = PopulationVector::new(header);
        pv.weights = weights;
        self.add_variable(Variable::PopulationVector(pv))
    }

    pub fn add_cont_var_with_floats_single_fragment(
        &mut self,
        cont_name: impl Into<String>,
        sampling_rate: f64,
        start_timestamp: f64,
        cont_values: Vec<f64>,
    ) -> Result<()> {
        if sampling_rate < 0.0 || sampling_rate > self.timestamp_frequency_hz {
            return Err(NexError::InvalidSamplingRate);
        }

        let name = cont_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Continuous as i32,
            name,
            sampling_rate,
            n_points_wave: cont_values.len() as u64,
            cont_data_type: 1,
            ad_to_mv: 1.0,
            ..Default::default()
        };
        let mut cont = ContinuousVariable::new(header);
        cont.fragment_timestamps = vec![start_timestamp];
        cont.fragment_indexes = vec![0];
        cont.continuous_values = cont_values;
        cont.calculate_fragment_counts_from_indexes();
        self.add_variable(Variable::Continuous(cont))
    }

    pub fn add_cont_single_fragment_values_int16(
        &mut self,
        cont_name: impl Into<String>,
        sampling_rate: f64,
        start_timestamp: f64,
        cont_values_as_int16: &[i16],
        raw_to_mv: f64,
        raw_offset: f64,
    ) -> Result<()> {
        if sampling_rate < 0.0 || sampling_rate > self.timestamp_frequency_hz {
            return Err(NexError::InvalidSamplingRate);
        }

        let values: Vec<f64> = cont_values_as_int16
            .iter()
            .map(|&v| v as f64 * raw_to_mv + raw_offset)
            .collect();

        let name = cont_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Continuous as i32,
            name,
            sampling_rate,
            n_points_wave: values.len() as u64,
            ad_to_mv: raw_to_mv,
            mv_offset: raw_offset,
            cont_data_type: 0,
            ..Default::default()
        };
        let mut cont = ContinuousVariable::new(header);
        cont.fragment_timestamps = vec![start_timestamp];
        cont.fragment_indexes = vec![0];
        cont.continuous_values = values;
        cont.calculate_fragment_counts_from_indexes();
        cont.hash_cont_values();
        self.add_variable(Variable::Continuous(cont))
    }

    pub fn add_cont_var_with_floats_all_timestamps(
        &mut self,
        cont_name: impl Into<String>,
        sampling_rate: f64,
        all_timestamps: Vec<f64>,
        cont_values: Vec<f64>,
    ) -> Result<()> {
        if sampling_rate < 0.0 || sampling_rate > self.timestamp_frequency_hz {
            return Err(NexError::InvalidSamplingRate);
        }
        if all_timestamps.len() != cont_values.len() {
            return Err(NexError::InvalidTimestampsAndValues);
        }

        let name = cont_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Continuous as i32,
            name,
            sampling_rate,
            n_points_wave: cont_values.len() as u64,
            cont_data_type: 1,
            ..Default::default()
        };
        let mut cont = ContinuousVariable::new(header);
        cont.fragment_timestamps = all_timestamps;
        cont.continuous_values = cont_values;
        cont.calculate_fragments_from_all_timestamps(self.timestamp_frequency_hz);
        cont.calculate_fragment_counts_from_indexes();
        self.add_variable(Variable::Continuous(cont))
    }

    pub fn add_wave_var_with_floats(
        &mut self,
        wave_name: impl Into<String>,
        sampling_rate: f64,
        timestamps: Vec<f64>,
        wave_values: Vec<Vec<f32>>,
    ) -> Result<()> {
        if sampling_rate < 0.0 || sampling_rate > self.timestamp_frequency_hz {
            return Err(NexError::InvalidSamplingRate);
        }

        let name = wave_name.into();
        let header = VariableHeader {
            var_type: NexFileVarType::Waveform as i32,
            name,
            sampling_rate,
            cont_data_type: 1,
            ..Default::default()
        };
        let mut wave = WaveformVariable::new(header);
        wave.timestamps = Timestamps::from(timestamps);
        wave.set_from_nested(wave_values)?;
        wave.assign_num_points_wave()?;
        self.add_variable(Variable::Waveform(wave))
    }
}

impl Index<&str> for FileData {
    type Output = Variable;

    fn index(&self, name: &str) -> &Self::Output {
        self.get_variable(name)
            .unwrap_or_else(|_| panic!("variable \"{name}\" not found in file data"))
    }
}

impl IndexMut<&str> for FileData {
    fn index_mut(&mut self, name: &str) -> &mut Self::Output {
        self.get_variable_mut(name)
            .unwrap_or_else(|_| panic!("variable \"{name}\" not found in file data"))
    }
}
