use std::collections::HashSet;

use ecow::EcoVec;
use typst::eval::Module;

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
    nowarn: EcoVec<Module>,
    nowarn_set: HashSet<u128>,
}

impl Tracer {
    /// The maximum number of inspected values.
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

    /// Get the values for the inspected span.
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
    /// The inspected span if it is part of the given source file.
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

    /// Add a warning. If the warning was emitted by a package that has its warnings suppressed, no warning is added.
    pub fn warn(&mut self, warning: SourceDiagnostic) {
        if let Some(package) = warning.emitter.map(|e| e.package()) {
            if let Some(package_name) = package.map(|p| &p.name) {
                if self.nowarn_set.contains(&hash128(package_name)) {
                    // TODO: how do we avoid conflicts of packages with the same name? currently, namespace is ignored, so is version.
                    return;
                }
            }
        }

        // Check if warning is a duplicate.
        let hash = hash128(&(&warning.span, &warning.message));
        if self.warnings_set.insert(hash) {
            self.warnings.push(warning);
        }
    }

    /// Stores that warnings from the given module must not be stored.
    /// TODO: Should this be allowed to be invoked by packages? This could be an anti-pattern.
    pub fn suppress_warnings_for(&mut self, module: Module) {
        let hash = hash128(module.name());
        if self.nowarn_set.insert(hash) {
            self.nowarn.push(module);
        }
    }
}
