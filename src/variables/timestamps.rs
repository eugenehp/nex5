#[cfg(not(feature = "std"))]
use crate::compat::prelude::*;

/// Spike/event timestamps stored as `f64` or compact `f32` seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Timestamps {
    inner: TimestampStorage,
}

#[derive(Debug, Clone, PartialEq)]
enum TimestampStorage {
    F64(Vec<f64>),
    F32(Vec<f32>),
}

impl Timestamps {
    pub fn new() -> Self {
        Self {
            inner: TimestampStorage::F64(Vec::new()),
        }
    }

    pub fn from_f64(values: Vec<f64>) -> Self {
        Self {
            inner: TimestampStorage::F64(values),
        }
    }

    pub fn from_f64_compact(values: Vec<f64>) -> Self {
        Self {
            inner: TimestampStorage::F32(values.into_iter().map(|v| v as f32).collect()),
        }
    }

    pub fn len(&self) -> usize {
        match &self.inner {
            TimestampStorage::F64(v) => v.len(),
            TimestampStorage::F32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_compact(&self) -> bool {
        matches!(self.inner, TimestampStorage::F32(_))
    }

    pub fn as_f64_vec(&self) -> Vec<f64> {
        match &self.inner {
            TimestampStorage::F64(v) => v.clone(),
            TimestampStorage::F32(v) => v.iter().map(|&t| f64::from(t)).collect(),
        }
    }

    pub fn as_f64_slice(&self) -> Vec<f64> {
        self.as_f64_vec()
    }

    pub fn iter_f64(&self) -> TimestampsIter<'_> {
        TimestampsIter {
            storage: &self.inner,
            index: 0,
        }
    }

    pub fn get(&self, index: usize) -> Option<f64> {
        match &self.inner {
            TimestampStorage::F64(v) => v.get(index).copied(),
            TimestampStorage::F32(v) => v.get(index).map(|&t| f64::from(t)),
        }
    }

    pub fn clear(&mut self) {
        match &mut self.inner {
            TimestampStorage::F64(v) => v.clear(),
            TimestampStorage::F32(v) => v.clear(),
        }
    }
}

impl Default for Timestamps {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<f64>> for Timestamps {
    fn from(values: Vec<f64>) -> Self {
        Self::from_f64(values)
    }
}

impl PartialEq<Vec<f64>> for Timestamps {
    fn eq(&self, other: &Vec<f64>) -> bool {
        self.iter_f64().eq(other.iter().copied())
    }
}

impl PartialEq<[f64]> for Timestamps {
    fn eq(&self, other: &[f64]) -> bool {
        self.iter_f64().eq(other.iter().copied())
    }
}

pub struct TimestampsIter<'a> {
    storage: &'a TimestampStorage,
    index: usize,
}

impl Iterator for TimestampsIter<'_> {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        match self.storage {
            TimestampStorage::F64(v) => {
                let value = v.get(self.index).copied()?;
                self.index += 1;
                Some(value)
            }
            TimestampStorage::F32(v) => {
                let value = v.get(self.index).copied()?;
                self.index += 1;
                Some(f64::from(value))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let rem = match self.storage {
            TimestampStorage::F64(v) => v.len().saturating_sub(self.index),
            TimestampStorage::F32(v) => v.len().saturating_sub(self.index),
        };
        (rem, Some(rem))
    }
}

impl<'de> serde::Deserialize<'de> for Timestamps {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = Vec::<f64>::deserialize(deserializer)?;
        Ok(Self::from_f64(values))
    }
}

impl serde::Serialize for Timestamps {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let values = self.as_f64_vec();
        values.serialize(serializer)
    }
}
