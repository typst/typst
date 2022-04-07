use crate::library::prelude::*;
use crate::library::text::{FontFamily, FontSize, TextNode, Toggle};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: usize,
    /// The heading's contents.
    pub body: Content,
}

#[node(showable)]
impl HeadingNode {
    /// The heading's font family. Just the normal text family if `auto`.
    #[property(referenced)]
    pub const FAMILY: Leveled<Smart<FontFamily>> = Leveled::Value(Smart::Auto);
    /// The color of text in the heading. Just the normal text color if `auto`.
    #[property(referenced)]
    pub const FILL: Leveled<Smart<Paint>> = Leveled::Value(Smart::Auto);
    /// The size of text in the heading.
    #[property(referenced)]
    pub const SIZE: Leveled<FontSize> = Leveled::Mapping(|level| {
        let upscale = (1.6 - 0.1 * level as f64).max(0.75);
        FontSize(Ratio::new(upscale).into())
    });
    /// Whether text in the heading is strengthend.
    #[property(referenced)]
    pub const STRONG: Leveled<bool> = Leveled::Value(true);
    /// Whether text in the heading is emphasized.
    #[property(referenced)]
    pub const EMPH: Leveled<bool> = Leveled::Value(false);
    /// Whether the heading is underlined.
    #[property(referenced)]
    pub const UNDERLINE: Leveled<bool> = Leveled::Value(false);
    /// The extra padding above the heading.
    #[property(referenced)]
    pub const ABOVE: Leveled<Length> = Leveled::Value(Length::zero());
    /// The extra padding below the heading.
    #[property(referenced)]
    pub const BELOW: Leveled<Length> = Leveled::Value(Length::zero());
    /// Whether the heading is block-level.
    #[property(referenced)]
    pub const BLOCK: Leveled<bool> = Leveled::Value(true);

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            body: args.expect("body")?,
            level: args.named("level")?.unwrap_or(1),
        }))
    }
}

impl Show for HeadingNode {
    fn show(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        macro_rules! resolve {
            ($key:expr) => {
                styles.get($key).resolve(ctx, self.level)?
            };
        }

        let args = [
            Value::Int(self.level as i64),
            Value::Content(self.body.clone()),
        ];

        let mut body = styles
            .show::<Self, _>(ctx, args)?
            .unwrap_or_else(|| self.body.clone());

        let mut map = StyleMap::new();
        map.set(TextNode::SIZE, resolve!(Self::SIZE));

        if let Smart::Custom(family) = resolve!(Self::FAMILY) {
            map.set_family(family, styles);
        }

        if let Smart::Custom(fill) = resolve!(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if resolve!(Self::STRONG) {
            map.set(TextNode::STRONG, Toggle);
        }

        if resolve!(Self::EMPH) {
            map.set(TextNode::EMPH, Toggle);
        }

        let mut seq = vec![];
        if resolve!(Self::UNDERLINE) {
            body = body.underlined();
        }

        let above = resolve!(Self::ABOVE);
        if !above.is_zero() {
            seq.push(Content::Vertical(above.into()));
        }

        seq.push(body);

        let below = resolve!(Self::BELOW);
        if !below.is_zero() {
            seq.push(Content::Vertical(below.into()));
        }

        let mut content = Content::sequence(seq).styled_with_map(map);

        if resolve!(Self::BLOCK) {
            content = Content::block(content);
        }

        Ok(content)
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

impl<T: Cast + Clone> Leveled<T> {
    /// Resolve the value based on the level.
    pub fn resolve(&self, ctx: &mut Context, level: usize) -> TypResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Mapping(mapping) => mapping(level),
            Self::Func(func, span) => {
                let args = Args::from_values(*span, [Value::Int(level as i64)]);
                func.call(ctx, args)?.cast().at(*span)?
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
