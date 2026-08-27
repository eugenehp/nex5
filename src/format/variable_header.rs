use super::data_format::DataFormat;
use super::io_helpers::{
    read_bytes, read_f64_le, read_i32_le, read_i64_le, to_string, write_f64_le, write_i32_le,
    write_i64_le, write_padded_string, write_padding,
};
use crate::compat::String;
use crate::io_ext::{IoResult, Read, Write};
use core::fmt;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VariableHeader {
    pub var_type: i32,
    pub version: i32,
    pub name: String,
    pub data_offset: u64,
    pub count: u64,
    pub ts_data_type: i32,
    pub cont_data_type: i32,
    pub sampling_rate: f64,
    pub units: String,
    pub ad_to_mv: f64,
    pub mv_offset: f64,
    pub n_points_wave: u64,
    pub pre_thr_time: f64,
    pub marker_data_type: i32,
    pub n_markers: i32,
    pub marker_length: i32,
    pub cont_frag_index_type: i32,
    pub gain: i32,
    pub filter: i32,
    pub wire: i32,
    pub unit: i32,
    pub x_pos: f64,
    pub y_pos: f64,
}

impl Default for VariableHeader {
    fn default() -> Self {
        Self {
            var_type: -1,
            version: 0,
            name: String::new(),
            data_offset: 0,
            count: 0,
            ts_data_type: 0,
            cont_data_type: 0,
            sampling_rate: 0.0,
            units: String::new(),
            ad_to_mv: 0.0,
            mv_offset: 0.0,
            n_points_wave: 0,
            pre_thr_time: 0.0,
            marker_data_type: 0,
            n_markers: 0,
            marker_length: 0,
            cont_frag_index_type: 0,
            gain: 0,
            filter: 0,
            wire: 0,
            unit: 0,
            x_pos: 0.0,
            y_pos: 0.0,
        }
    }
}

impl fmt::Display for VariableHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl VariableHeader {
    pub fn read_from_nex(reader: &mut impl Read) -> IoResult<Self> {
        let header = Self {
            var_type: read_i32_le(reader)?,
            version: read_i32_le(reader)?,
            name: to_string(&read_bytes(reader, 64)?, true),
            data_offset: read_i32_le(reader)? as u64,
            count: read_i32_le(reader)? as u64,
            wire: read_i32_le(reader)?,
            unit: read_i32_le(reader)?,
            gain: read_i32_le(reader)?,
            filter: read_i32_le(reader)?,
            x_pos: read_f64_le(reader)?,
            y_pos: read_f64_le(reader)?,
            sampling_rate: read_f64_le(reader)?,
            ad_to_mv: read_f64_le(reader)?,
            n_points_wave: read_i32_le(reader)? as u64,
            n_markers: read_i32_le(reader)?,
            marker_length: read_i32_le(reader)?,
            mv_offset: read_f64_le(reader)?,
            pre_thr_time: read_f64_le(reader)?,
            ts_data_type: 0,
            cont_data_type: 0,
            units: String::new(),
            marker_data_type: 0,
            cont_frag_index_type: 0,
        };
        let _padding = read_bytes(reader, 52)?;
        Ok(header)
    }

    pub fn read_from_nex5(reader: &mut impl Read) -> IoResult<Self> {
        let mut header = Self {
            var_type: read_i32_le(reader)?,
            version: read_i32_le(reader)?,
            name: to_string(&read_bytes(reader, 64)?, false),
            data_offset: read_i64_le(reader)? as u64,
            count: read_i64_le(reader)? as u64,
            ts_data_type: read_i32_le(reader)?,
            cont_data_type: read_i32_le(reader)?,
            sampling_rate: read_f64_le(reader)?,
            units: to_string(&read_bytes(reader, 32)?, false),
            ad_to_mv: read_f64_le(reader)?,
            mv_offset: read_f64_le(reader)?,
            n_points_wave: read_i64_le(reader)? as u64,
            pre_thr_time: read_f64_le(reader)?,
            marker_data_type: read_i32_le(reader)?,
            n_markers: read_i32_le(reader)?,
            marker_length: read_i32_le(reader)?,
            cont_frag_index_type: read_i32_le(reader)?,
            gain: 0,
            filter: 0,
            wire: 0,
            unit: 0,
            x_pos: 0.0,
            y_pos: 0.0,
        };

        let _padding = read_bytes(reader, 60)?;

        if header.cont_data_type == 1 {
            header.ad_to_mv = 1.0;
            header.mv_offset = 0.0;
        }

        Ok(header)
    }

    pub fn write_to_nex(&self, writer: &mut impl Write) -> IoResult<()> {
        write_i32_le(writer, self.var_type)?;
        write_i32_le(writer, self.version)?;
        write_padded_string(writer, &self.name, 64)?;
        write_i32_le(writer, self.data_offset as i32)?;
        write_i32_le(writer, self.count as i32)?;
        write_i32_le(writer, self.wire)?;
        write_i32_le(writer, self.unit)?;
        write_i32_le(writer, self.gain)?;
        write_i32_le(writer, self.filter)?;
        write_f64_le(writer, self.x_pos)?;
        write_f64_le(writer, self.y_pos)?;
        write_f64_le(writer, self.sampling_rate)?;
        write_f64_le(writer, self.ad_to_mv)?;
        write_i32_le(writer, self.n_points_wave as i32)?;
        write_i32_le(writer, self.n_markers)?;
        write_i32_le(writer, self.marker_length)?;
        write_f64_le(writer, self.mv_offset)?;
        write_f64_le(writer, self.pre_thr_time)?;
        write_padding(writer, 52)
    }

    pub fn write_to_nex5(&self, writer: &mut impl Write) -> IoResult<()> {
        write_i32_le(writer, self.var_type)?;
        write_i32_le(writer, self.version)?;
        write_padded_string(writer, &self.name, 64)?;
        write_i64_le(writer, self.data_offset as i64)?;
        write_i64_le(writer, self.count as i64)?;
        write_i32_le(writer, self.ts_data_type)?;
        write_i32_le(writer, self.cont_data_type)?;
        write_f64_le(writer, self.sampling_rate)?;
        write_padded_string(writer, &self.units, 32)?;
        write_f64_le(writer, self.ad_to_mv)?;
        write_f64_le(writer, self.mv_offset)?;
        write_i64_le(writer, self.n_points_wave as i64)?;
        write_f64_le(writer, self.pre_thr_time)?;
        write_i32_le(writer, self.marker_data_type)?;
        write_i32_le(writer, self.n_markers)?;
        write_i32_le(writer, self.marker_length)?;
        write_i32_le(writer, self.cont_frag_index_type)?;
        write_padding(writer, 60)
    }

    pub fn cont_data_pars(&self) -> (DataFormat, f64, f64) {
        if self.cont_data_type == 1 {
            (DataFormat::Float32, 1.0, 0.0)
        } else {
            (DataFormat::Int16, self.ad_to_mv, self.mv_offset)
        }
    }

    pub fn timestamp_data_format(&self) -> DataFormat {
        if self.ts_data_type == 1 {
            DataFormat::Int64
        } else {
            DataFormat::Int32
        }
    }

    pub fn bytes_in_timestamp(&self) -> usize {
        if self.ts_data_type == 0 {
            4
        } else {
            8
        }
    }

    pub fn bytes_in_cont_value(&self) -> usize {
        if self.cont_data_type == 0 {
            2
        } else {
            4
        }
    }

    pub fn bytes_in_fragment_index(&self) -> usize {
        4
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn nex_var_header_write_size_and_version() {
        let mut buf = Vec::new();
        let h = VariableHeader {
            var_type: 0,
            version: 102,
            name: "nr".to_string(),
            ..Default::default()
        };
        h.write_to_nex(&mut buf).unwrap();
        assert_eq!(buf.len(), 208);
        assert_eq!(i32::from_le_bytes(buf[0..4].try_into().unwrap()), 0);
        assert_eq!(i32::from_le_bytes(buf[4..8].try_into().unwrap()), 102);
        assert_eq!(&buf[8..10], b"nr");
    }
}
