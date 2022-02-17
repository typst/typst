//! Document-structuring section headings.

use super::prelude::*;
use super::{FontFamily, TextNode};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: usize,
    /// The heading's contents.
    pub body: Template,
}

#[class]
impl HeadingNode {
    /// The heading's font family. Just the normal text family if `auto`.
    pub const FAMILY: Leveled<Smart<FontFamily>> = Leveled::Value(Smart::Auto);
    /// The color of text in the heading. Just the normal text color if `auto`.
    pub const FILL: Leveled<Smart<Paint>> = Leveled::Value(Smart::Auto);
    /// The size of text in the heading.
    pub const SIZE: Leveled<Linear> = Leveled::Mapping(|level| {
        let upscale = (1.6 - 0.1 * level as f64).max(0.75);
        Relative::new(upscale).into()
    });
    /// Whether text in the heading is strengthend.
    pub const STRONG: Leveled<bool> = Leveled::Value(true);
    /// Whether text in the heading is emphasized.
    pub const EMPH: Leveled<bool> = Leveled::Value(false);
    /// Whether the heading is underlined.
    pub const UNDERLINE: Leveled<bool> = Leveled::Value(false);
    /// The extra padding above the heading.
    pub const ABOVE: Leveled<Length> = Leveled::Value(Length::zero());
    /// The extra padding below the heading.
    pub const BELOW: Leveled<Length> = Leveled::Value(Length::zero());

    fn construct(_: &mut Vm, args: &mut Args) -> TypResult<Template> {
        Ok(Template::show(Self {
            body: args.expect("body")?,
            level: args.named("level")?.unwrap_or(1),
        }))
    }
}

impl Show for HeadingNode {
    fn show(&self, vm: &mut Vm, styles: StyleChain) -> TypResult<Template> {
        macro_rules! resolve {
            ($key:expr) => {
                styles.get_cloned($key).resolve(vm, self.level)?
            };
        }

        let mut map = StyleMap::new();
        map.set(TextNode::SIZE, resolve!(Self::SIZE));

        if let Smart::Custom(family) = resolve!(Self::FAMILY) {
            map.set(
                TextNode::FAMILY,
                std::iter::once(family)
                    .chain(styles.get_ref(TextNode::FAMILY).iter().cloned())
                    .collect(),
            );
        }

        if let Smart::Custom(fill) = resolve!(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if resolve!(Self::STRONG) {
            map.set(TextNode::STRONG, true);
        }

        if resolve!(Self::EMPH) {
            map.set(TextNode::EMPH, true);
        }

        let mut seq = vec![];
        let mut body = self.body.clone();
        if resolve!(Self::UNDERLINE) {
            body = body.underlined();
        }

        let above = resolve!(Self::ABOVE);
        if !above.is_zero() {
            seq.push(Template::Vertical(above.into()));
        }

        seq.push(body);

        let below = resolve!(Self::BELOW);
        if !below.is_zero() {
            seq.push(Template::Vertical(below.into()));
        }

        Ok(Template::block(
            Template::sequence(seq).styled_with_map(map),
        ))
    }
}

/// Either the value or a closure mapping to the value.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Leveled<T> {
    /// A bare value.
    Value(T),
    /// A simple mapping from a heading level to a value.
    Mapping(fn(usize) -> T),
    /// A closure mapping from a heading level to a value.
    Func(Func, Span),
}

impl<T: Cast> Leveled<T> {
    /// Resolve the value based on the level.
    pub fn resolve(self, vm: &mut Vm, level: usize) -> TypResult<T> {
        Ok(match self {
            Self::Value(value) => value,
            Self::Mapping(mapping) => mapping(level),
            Self::Func(func, span) => {
                let args = Args {
                    span,
                    items: vec![Arg {
                        span,
                        name: None,
                        value: Spanned::new(Value::Int(level as i64), span),
                    }],
                };
                func.call(vm, args)?.cast().at(span)?
            }
        })
    }
}

impl<T: Cast> Cast<Spanned<Value>> for Leveled<T> {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(&value.v, Value::Func(_)) || T::is(&value.v)
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            v => T::cast(v)
                .map(Self::Value)
                .map_err(|msg| with_alternative(msg, "function")),
        }
    }
}
