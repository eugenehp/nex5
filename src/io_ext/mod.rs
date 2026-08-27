//! I/O traits and helpers for `std` and `no_std` builds.

#[cfg(feature = "std")]
pub use std::io::{Cursor, Read, Result as IoResult, Seek, SeekFrom, Write};

#[cfg(not(feature = "std"))]
mod slice;

#[cfg(not(feature = "std"))]
pub use slice::{Cursor, Error, IoResult, Read, Seek, SeekFrom, VecCursor, Write};
