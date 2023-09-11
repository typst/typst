use std::collections::HashSet;

use ecow::EcoVec;

use super::Value;
use crate::diag::SourceDiagnostic;
use crate::syntax::{FileId, Span};
use crate::util::hash128;

/// Traces warnings and which values existed for an expression at a span.
#[derive(Default, Clone)]
pub struct Tracer {
    inspected: Option<Span>,
    values: EcoVec<Value>,
    warnings: EcoVec<SourceDiagnostic>,
    warnings_set: HashSet<u128>,
}

impl Tracer {
    /// The maximum number of inspeted values.
    pub const MAX_VALUES: usize = 10;

    /// Create a new tracer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a span as inspected. All values observed for this span can be
    /// retrieved via `values` later.
    pub fn inspect(&mut self, span: Span) {
        self.inspected = Some(span);
    }

    /// Get the values for the inspeted span.
    pub fn values(self) -> EcoVec<Value> {
        self.values
    }

    /// Get the stored warnings.
    pub fn warnings(self) -> EcoVec<SourceDiagnostic> {
        self.warnings
    }
}

#[comemo::track]
impl Tracer {
    /// The inspeted span if it is part of the given source file.
    pub fn inspected(&self, id: FileId) -> Option<Span> {
        if self.inspected.and_then(Span::id) == Some(id) {
            self.inspected
        } else {
            None
        }
    }

    /// Trace a value for the span.
    pub fn value(&mut self, v: Value) {
        if self.values.len() < Self::MAX_VALUES {
            self.values.push(v);
        }
    }

    /// Add a warning.
    pub fn warn(&mut self, warning: SourceDiagnostic) {
        // Check if warning is a duplicate.
        let hash = hash128(&(&warning.span, &warning.message));
        if self.warnings_set.insert(hash) {
            self.warnings.push(warning);
        }
    }
}
