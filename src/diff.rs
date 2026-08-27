//! Compare two [`FileData`] sessions.

use crate::compat::{String, Vec};
use crate::file_data::FileData;
use crate::variables::{NexFileVarType, Variable};

/// Summary of differences between two in-memory files.
#[derive(Debug, Clone, PartialEq)]
pub struct FileDataDiff {
    pub only_in_a: Vec<String>,
    pub only_in_b: Vec<String>,
    pub changed: Vec<VariableDiff>,
}

/// Per-variable difference details.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDiff {
    pub name: String,
    pub kind: String,
    pub count_a: u64,
    pub count_b: u64,
    pub max_timestamp_delta_sec: f64,
}

impl FileData {
    /// Compare variable names and payload counts against another session.
    pub fn diff(&self, other: &FileData) -> FileDataDiff {
        let mut only_in_a = Vec::new();
        let mut only_in_b = Vec::new();
        let mut changed = Vec::new();

        for name in self.variable_names() {
            if other.get_variable(&name).is_err() {
                only_in_a.push(name);
            }
        }
        for name in other.variable_names() {
            if self.get_variable(&name).is_err() {
                only_in_b.push(name);
            }
        }

        for name in self.variable_names() {
            let Ok(a) = self.get_variable(&name) else { continue };
            let Ok(b) = other.get_variable(&name) else { continue };
            let count_a = a.count_for_header();
            let count_b = b.count_for_header();
            let max_delta = (a.maximum_timestamp() - b.maximum_timestamp()).abs();
            if count_a != count_b || max_delta > f64::EPSILON || a.var_type() != b.var_type() {
                changed.push(VariableDiff {
                    name,
                    kind: variable_kind_label(a),
                    count_a,
                    count_b,
                    max_timestamp_delta_sec: max_delta,
                });
            }
        }

        FileDataDiff {
            only_in_a,
            only_in_b,
            changed,
        }
    }
}

fn variable_kind_label(var: &Variable) -> String {
    NexFileVarType::from_i32(var.var_type())
        .map(|t| t.name().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
