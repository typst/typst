use std::fmt::{Display, Write};
use std::ops::ControlFlow;

use comemo::Track;
use ecow::EcoString;
use indexmap::IndexMap;
use typst::engine::{Engine, Route, Sink, Traced};
use typst::foundations::{Scope, Value};
use typst::introspection::EmptyIntrospector;
use typst::syntax::{LinkedNode, SyntaxMode};
use typst::text::{
    AxisValue, FontAxis, FontFlags, FontInfo, FontStretch, FontStyle, FontWeight,
    StandardAxes,
};
use typst::utils::Protected;
use typst_utils::Scalar;

use crate::IdeWorld;

/// Create a temporary engine and run a task on it.
pub fn with_engine<F, T>(world: &dyn IdeWorld, f: F) -> T
where
    F: FnOnce(&mut Engine) -> T,
{
    let introspector = EmptyIntrospector;
    let traced = Traced::default();
    let mut sink = Sink::new();
    let mut engine = Engine {
        library: world.library(),
        world: world.upcast().track(),
        introspector: Protected::new(introspector.track()),
        traced: traced.track(),
        sink: sink.track_mut(),
        route: Route::default(),
    };

    f(&mut engine)
}

/// Create a short description of a font family.
pub fn summarize_font_family<'a>(
    variants: impl IntoIterator<Item = &'a FontInfo>,
) -> EcoString {
    use typst::text::Tag;

    let mut count = 0;
    let mut variable = false;
    let mut has_italics = false;
    let mut has_obliques = false;
    let mut supports_opsz = false;
    let mut weight_range = MetricRange::default();
    let mut stretch_range = MetricRange::default();
    let mut variations = IndexMap::<Tag, MetricRange<Scalar>>::new();

    for info in variants {
        let axes = StandardAxes::parse(&info.axes);

        variable |= info.flags.contains(FontFlags::VARIABLE);
        has_italics |= info.variant.style == FontStyle::Italic || axes.ital.is_some();
        has_obliques |= info.variant.style == FontStyle::Oblique || axes.slnt.is_some();
        supports_opsz |= axes.opsz.is_some();

        weight_range.expand(info.variant.weight);
        stretch_range.expand(info.variant.stretch);

        if let Some(axis) = axes.wght {
            weight_range.expand_axis(axis, FontWeight::from_wght);
        }

        if let Some(axis) = axes.wdth {
            stretch_range.expand_axis(axis, FontStretch::from_wdth);
        }

        for axis in &info.axes {
            if !StandardAxes::knows(axis.tag) {
                variations
                    .entry(axis.tag)
                    .or_default()
                    .expand_axis(axis, |v| Scalar::new(v.0.into()));
            }
        }

        count += 1;
    }

    let mut detail = EcoString::new();

    if variable {
        write!(detail, "Variable.").unwrap();
    } else {
        write!(detail, "{count} variant{}.", if count == 1 { "" } else { "s" }).unwrap();
    }

    if let Some(range) = weight_range.display_if_not_default() {
        write!(detail, " Weight {range}.").unwrap();
    }

    if let Some(range) = stretch_range.display_if_not_default() {
        write!(detail, " Stretch {range}.").unwrap();
    }

    if has_italics {
        detail.push_str(" Has italics.");
    }

    if has_obliques {
        detail.push_str(" Has obliques.");
    }

    if supports_opsz {
        detail.push_str(" Supports optical sizing.");
    }

    for (tag, range) in variations {
        if let Some(range) = range.display() {
            write!(detail, " {} {range}.", tag.to_str_lossy()).unwrap();
        }
    }

    detail
}

/// A font metric with a minimum and maximum value. Internally tracks an
/// uninitialized state.
struct MetricRange<T>(Option<(T, T)>);

impl<T> MetricRange<T>
where
    T: Display + Copy + Ord,
{
    /// Makes sure the range includes the value.
    fn expand(&mut self, value: T) {
        self.expand_range(value, value);
    }

    /// Makes sure the range includes all values from the given variation axis.
    fn expand_axis(&mut self, axis: &FontAxis, f: impl Fn(AxisValue) -> T) {
        self.expand_range(f(axis.min), f(axis.max));
    }

    /// Makes sure the range includes the minimum and maximum value.
    fn expand_range(&mut self, min: T, max: T) {
        self.0 = Some(match self.0 {
            None => (min, max),
            Some((prev_min, prev_max)) => (prev_min.min(min), prev_max.max(max)),
        });
    }

    /// Displays the range if initialized and not the default for that metric.
    fn display(self) -> Option<impl Display> {
        self.0.map(|(min, max)| {
            typst_utils::display(move |f| {
                if min == max { write!(f, "{min}") } else { write!(f, "{min}–{max}") }
            })
        })
    }

    /// Displays the range if initialized and not the default for that metric.
    fn display_if_not_default(self) -> Option<impl Display>
    where
        T: Default,
    {
        let default = T::default();
        Self(self.0.filter(|&(min, max)| min != default || max != default)).display()
    }
}

impl<T> Default for MetricRange<T> {
    fn default() -> Self {
        Self(None)
    }
}

/// The global definitions at the given node.
pub fn globals<'a>(world: &'a dyn IdeWorld, leaf: &LinkedNode) -> &'a Scope {
    let library = world.library();
    if leaf.mode_after() == Some(SyntaxMode::Math) {
        library.math.scope()
    } else {
        library.global.scope()
    }
}

/// Checks whether the given value or any of its constituent parts satisfy the
/// predicate.
pub fn check_value_recursively(
    value: &Value,
    predicate: impl Fn(&Value) -> bool,
) -> bool {
    let mut searcher = Searcher { steps: 0, predicate, max_steps: 1000 };
    match searcher.find(value) {
        ControlFlow::Break(matching) => matching,
        ControlFlow::Continue(()) => false,
    }
}

/// Recursively searches for a value that passes the filter, but without
/// exceeding a maximum number of search steps.
struct Searcher<F> {
    max_steps: usize,
    steps: usize,
    predicate: F,
}

impl<F> Searcher<F>
where
    F: Fn(&Value) -> bool,
{
    fn find(&mut self, value: &Value) -> ControlFlow<bool> {
        if (self.predicate)(value) {
            return ControlFlow::Break(true);
        }

        if self.steps > self.max_steps {
            return ControlFlow::Break(false);
        }

        self.steps += 1;

        match value {
            Value::Dict(dict) => {
                self.find_iter(dict.iter().map(|(_, v)| v))?;
            }
            Value::Content(content) => {
                self.find_iter(content.fields().iter().map(|(_, v)| v))?;
            }
            Value::Module(module) => {
                self.find_iter(module.scope().iter().map(|(_, b)| b.read()))?;
            }
            _ => {}
        }

        ControlFlow::Continue(())
    }

    fn find_iter<'a>(
        &mut self,
        iter: impl Iterator<Item = &'a Value>,
    ) -> ControlFlow<bool> {
        for item in iter {
            self.find(item)?;
        }
        ControlFlow::Continue(())
    }
}

#[cfg(test)]
mod tests {
    use typst::text::{FontBook, FontInfo};

    use super::*;

    #[test]
    fn test_summarize_font_family() {
        let book = FontBook::from_infos(
            typst_dev_assets::fonts().filter_map(|data| FontInfo::new(data, 0)),
        );

        let summarize = |family: &str| {
            summarize_font_family(
                book.select_family(&family.to_lowercase())
                    .map(|id| book.info(id).unwrap()),
            )
        };

        // Static.
        assert_eq!(summarize("Cascadia Mono"), "2 variants. Weight 400–700.");
        assert_eq!(summarize("HK Grotesk"), "16 variants. Weight 100–900. Has italics.");
        assert_eq!(
            summarize("IBM Plex Sans"),
            "5 variants. Weight 300–700. Stretch 75%–100%."
        );

        // Variable.
        assert_eq!(summarize("Cantarell"), "Variable. Weight 100–800.");
        assert_eq!(
            summarize("Fraunces"),
            "Variable. Weight 100–900. Has italics. \
             Supports optical sizing. SOFT 0–100. WONK 0–1."
        );
        assert_eq!(
            summarize("Mona Sans"),
            "Variable. Weight 200–900. Stretch 75%–125%. Has italics. \
             Supports optical sizing."
        );
    }
}
