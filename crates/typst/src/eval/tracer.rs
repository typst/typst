use std::collections::HashSet;

use ecow::{eco_vec, EcoVec};
use typst_syntax::{FileId, Span};

use crate::{diag::SourceDiagnostic, util::hash128};

use super::Value;

/// Traces warnings and which values existed for an expression at a span.
#[derive(Default, Clone)]
pub struct Tracer {
    pub span: Option<Span>,
    pub values: EcoVec<Value>,

    pub warnings: EcoVec<SourceDiagnostic>,
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
    pub fn finish(self) -> EcoVec<Value> {
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

    /// Add a warning
    pub fn warn(&mut self, warning: SourceDiagnostic) {
        // check if warning is a duplicate
        let hash = hash128(&(warning.span, warning.message.clone()));

        if !self.warnings_set.contains(&hash) {
            self.warnings_set.insert(hash);

            self.warnings.push(warning);
        }
    }
}
