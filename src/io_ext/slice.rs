use crate::compat::Vec;
use core::cmp::min;
use core::fmt;

pub type IoResult<T> = Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    UnexpectedEof,
    WriteZero,
    InvalidInput,
    Other,
}

#[derive(Debug, Clone)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind) -> Self {
        Self { kind }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.kind)
    }
}

pub trait Read {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize>;
    fn read_exact(&mut self, buf: &mut [u8]) -> IoResult<()> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.read(&mut buf[filled..]) {
                Ok(0) => return Err(Error::new(ErrorKind::UnexpectedEof)),
                Ok(n) => filled += n,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

pub trait Write {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize>;
    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        let mut written = 0;
        while written < buf.len() {
            match self.write(&buf[written..]) {
                Ok(0) => return Err(Error::new(ErrorKind::WriteZero)),
                Ok(n) => written += n,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn flush(&mut self) -> IoResult<()> {
        Ok(())
    }
}

pub trait Seek {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64>;
    fn stream_position(&mut self) -> IoResult<u64> {
        self.seek(SeekFrom::Current(0))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T> Cursor<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    #[allow(dead_code)]
    pub fn into_inner(self) -> T {
        self.inner
    }

    #[allow(dead_code)]
    pub fn position(&self) -> u64 {
        self.pos
    }
}

impl Cursor<&[u8]> {
    fn len(&self) -> u64 {
        self.inner.len() as u64
    }
}

impl Read for Cursor<&[u8]> {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        let pos = min(self.pos, self.len()) as usize;
        let avail = &self.inner[pos..];
        let n = min(buf.len(), avail.len());
        buf[..n].copy_from_slice(&avail[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for Cursor<&[u8]> {
    fn seek(&mut self, style: SeekFrom) -> IoResult<u64> {
        let len = self.len();
        let new_pos = match style {
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
        self.pos = min(new_pos, len);
        Ok(self.pos)
    }
}

impl Write for Cursor<&mut [u8]> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let pos = min(self.pos, self.inner.len() as u64) as usize;
        let avail = &mut self.inner[pos..];
        let n = min(buf.len(), avail.len());
        avail[..n].copy_from_slice(&buf[..n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl Write for Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }
}

impl Seek for Vec<u8> {
    fn seek(&mut self, style: SeekFrom) -> IoResult<u64> {
        let len = self.len() as u64;
        let new_pos = match style {
            SeekFrom::Start(off) => off,
            SeekFrom::Current(off) => {
                if off >= 0 {
                    len.saturating_add(off as u64)
                } else {
                    len.saturating_sub((-off) as u64)
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
        Ok(new_pos)
    }
}

/// Growable buffer writer with seek/patch support (for in-memory `.nex` output).
pub struct VecCursor {
    buf: Vec<u8>,
    pos: u64,
}

impl VecCursor {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            pos: 0,
        }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }
}

impl Default for VecCursor {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for VecCursor {
    fn write(&mut self, data: &[u8]) -> IoResult<usize> {
        let pos = self.pos as usize;
        let end = pos.saturating_add(data.len());
        if end > self.buf.len() {
            self.buf.resize(end, 0);
        }
        self.buf[pos..end].copy_from_slice(data);
        self.pos = end as u64;
        Ok(data.len())
    }
}

impl Seek for VecCursor {
    fn seek(&mut self, style: SeekFrom) -> IoResult<u64> {
        let len = self.buf.len() as u64;
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

    fn stream_position(&mut self) -> IoResult<u64> {
        Ok(self.pos)
    }
}
