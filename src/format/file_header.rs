use super::io_helpers::{
    read_bytes, read_f64_le, read_i32_le, read_i64_le, to_string, write_f64_le, write_i32_le,
    write_i64_le, write_padded_string, write_padding,
};
use crate::compat::String;
use crate::error::{NexError, Result};
use crate::format::{NEX5_MAGIC, NEX_MAGIC};
use crate::io_ext::{IoResult, Read, Write};

#[derive(Debug, Clone, PartialEq)]
pub struct FileHeader {
    pub magic_number: i32,
    pub nex_file_version: i32,
    pub comment: String,
    pub frequency: f64,
    pub beg_ticks: i64,
    pub end_ticks: i64,
    pub num_vars: i32,
    pub meta_offset: u64,
    pub beg_seconds: f64,
    pub end_seconds: f64,
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            magic_number: 0,
            nex_file_version: 0,
            comment: String::new(),
            frequency: 0.0,
            beg_ticks: 0,
            end_ticks: 0,
            num_vars: 0,
            meta_offset: 0,
            beg_seconds: 0.0,
            end_seconds: 0.0,
        }
    }
}

impl FileHeader {
    pub fn read_from_nex(reader: &mut impl Read) -> Result<Self> {
        let magic_number = read_i32_le(reader).map_err(|_| NexError::InvalidNexHeader)?;
        if magic_number != NEX_MAGIC {
            return Err(NexError::InvalidNexHeader);
        }
        let nex_file_version = read_i32_le(reader).map_err(|_| NexError::InvalidNexHeader)?;
        let comment = to_string(
            &read_bytes(reader, 256).map_err(|_| NexError::InvalidNexHeader)?,
            false,
        );
        let frequency = read_f64_le(reader).map_err(|_| NexError::InvalidNexHeader)?;
        let beg_ticks = read_i32_le(reader).map_err(|_| NexError::InvalidNexHeader)? as i64;
        let end_ticks = read_i32_le(reader).map_err(|_| NexError::InvalidNexHeader)? as i64;
        let num_vars = read_i32_le(reader).map_err(|_| NexError::InvalidNexHeader)?;
        let _padding = read_bytes(reader, 260).map_err(|_| NexError::InvalidNexHeader)?;

        Ok(Self {
            magic_number,
            nex_file_version,
            comment,
            frequency,
            beg_ticks,
            end_ticks,
            num_vars,
            meta_offset: 0,
            beg_seconds: beg_ticks as f64 / frequency,
            end_seconds: end_ticks as f64 / frequency,
        })
    }

    pub fn read_from_nex5(reader: &mut impl Read) -> Result<Self> {
        let magic_number = read_i32_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        if magic_number != NEX5_MAGIC {
            return Err(NexError::InvalidNex5Header);
        }
        let nex_file_version = read_i32_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let comment = to_string(
            &read_bytes(reader, 256).map_err(|_| NexError::InvalidNex5Header)?,
            false,
        );
        let frequency = read_f64_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let beg_ticks = read_i64_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let num_vars = read_i32_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let meta_offset =
            super::io_helpers::read_u64_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let end_ticks = read_i64_le(reader).map_err(|_| NexError::InvalidNex5Header)?;
        let _padding = read_bytes(reader, 56).map_err(|_| NexError::InvalidNex5Header)?;

        Ok(Self {
            magic_number,
            nex_file_version,
            comment,
            frequency,
            beg_ticks,
            end_ticks,
            num_vars,
            meta_offset,
            beg_seconds: beg_ticks as f64 / frequency,
            end_seconds: end_ticks as f64 / frequency,
        })
    }

    pub fn write_to_nex(&self, writer: &mut impl Write) -> IoResult<()> {
        write_i32_le(writer, self.magic_number)?;
        write_i32_le(writer, self.nex_file_version)?;
        write_padded_string(writer, &self.comment, 256)?;
        write_f64_le(writer, self.frequency)?;
        write_i32_le(writer, self.beg_ticks as i32)?;
        write_i32_le(writer, self.end_ticks as i32)?;
        write_i32_le(writer, self.num_vars)?;
        write_padding(writer, 260)
    }

    pub fn write_to_nex5(&self, writer: &mut impl Write) -> IoResult<()> {
        write_i32_le(writer, self.magic_number)?;
        write_i32_le(writer, self.nex_file_version)?;
        write_padded_string(writer, &self.comment, 256)?;
        write_f64_le(writer, self.frequency)?;
        write_i64_le(writer, self.beg_ticks)?;
        write_i32_le(writer, self.num_vars)?;
        super::io_helpers::write_u64_le(writer, self.meta_offset)?;
        write_i64_le(writer, self.end_ticks)?;
        write_padding(writer, 56)
    }
}
