mod data_format;
mod file_header;
mod io_helpers;
mod variable_header;

pub use data_format::{
    for_each_f64_chunk, for_each_timestamp_chunk, read_f32_vec, read_f64_vec, read_i16_vec,
    read_timestamps_as_f64, read_timestamps_range, read_u32_vec, write_f64_vec,
    write_timestamps_as_i32, write_timestamps_as_i64, write_u32_vec, DataFormat,
};
pub use file_header::FileHeader;
pub use io_helpers::{
    read_bytes, to_string, write_padded_string, write_u32_le, write_u64_le,
};
pub use variable_header::VariableHeader;

pub const NEX_MAGIC: i32 = 827_868_494;
pub const NEX5_MAGIC: i32 = 894_977_358;

pub const NEX_FILE_HEADER_SIZE: u64 = 544;
pub const NEX5_FILE_HEADER_SIZE: u64 = 356;
pub const NEX_VAR_HEADER_SIZE: u64 = 208;
pub const NEX5_VAR_HEADER_SIZE: u64 = 244;
