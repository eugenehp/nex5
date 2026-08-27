//! Memory-mapped file backend for zero-copy reads (`mmap` feature).

use crate::io_ext::{IoResult, Read, Seek, SeekFrom};
use std::path::Path;

#[cfg(feature = "mmap")]
use memmap2::Mmap;

/// Read+Seek over a memory-mapped file slice.
#[cfg(feature = "mmap")]
pub struct MmapReader {
    map: Mmap,
    pos: u64,
}

#[cfg(feature = "mmap")]
impl MmapReader {
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path.as_ref())?;
        let map = unsafe { Mmap::map(&file)? };
        Ok(Self { map, pos: 0 })
    }

    pub fn from_mmap(map: Mmap) -> Self {
        Self { map, pos: 0 }
    }

    pub fn len(&self) -> u64 {
        self.map.len() as u64
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Borrow a sub-slice at an absolute file offset (zero-copy).
    pub fn slice_at(&self, offset: u64, len: usize) -> Option<&[u8]> {
        let start = offset as usize;
        let end = start.checked_add(len)?;
        self.map.get(start..end)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.map
    }
}

#[cfg(feature = "mmap")]
impl Read for MmapReader {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let pos = self.pos.min(self.len()) as usize;
        let avail = &self.map[pos..];
        let n = buf.len().min(avail.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

#[cfg(feature = "mmap")]
impl Seek for MmapReader {
    fn seek(&mut self, style: SeekFrom) -> IoResult<u64> {
        let len = self.len();
        self.pos = match style {
            SeekFrom::Start(off) => off,
            SeekFrom::Current(off) => {
                if off >= 0 {
                    self.pos.saturating_add(off as u64)
                } else {
                    self.pos.saturating_sub((-off) as u64)
                }
            }
            SeekFrom::End(off) => {
                if off >= 0 {
                    len.saturating_add(off as u64)
                } else {
                    len.saturating_sub((-off) as u64)
                }
            }
        };
        Ok(self.pos)
    }
}
