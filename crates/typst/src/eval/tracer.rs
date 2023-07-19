use std::collections::HashSet;

use ecow::{eco_vec, EcoVec};

use super::Value;
use crate::diag::SourceDiagnostic;
use crate::syntax::{FileId, Span};
use crate::util::hash128;

/// Traces warnings and which values existed for an expression at a span.
#[derive(Default, Clone)]
pub struct Tracer {
    span: Option<Span>,
    values: EcoVec<Value>,
    warnings: EcoVec<SourceDiagnostic>,
    warnings_set: HashSet<u128>,
}

impl Tracer {
    /// The maximum number of traced items.
    pub const MAX: usize = 10;

    /// Create a new tracer, possibly with a span under inspection.
    pub fn new(span: Option<Span>) -> Self {
        Self {
            span,
            values: eco_vec![],
            warnings: eco_vec![],
            warnings_set: HashSet::new(),
        }
    }

    /// Get the traced values.
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
    /// The traced span if it is part of the given source file.
    pub fn span(&self, id: FileId) -> Option<Span> {
        if self.span.map(Span::id) == Some(id) {
            self.span
        } else {
            None
        }
    }

    /// Trace a value for the span.
    pub fn trace(&mut self, v: Value) {
        if self.values.len() < Self::MAX {
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
