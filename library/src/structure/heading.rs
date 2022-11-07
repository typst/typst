use crate::layout::{BlockNode, BlockSpacing};
use crate::prelude::*;
use crate::text::{FontFamily, TextNode, TextSize};

/// A section heading.
#[derive(Debug, Hash)]
pub struct HeadingNode {
    /// The logical nesting depth of the section, starting from one. In the
    /// default style, this controls the text size of the heading.
    pub level: NonZeroUsize,
    /// The heading's contents.
    pub body: Content,
}

#[node(Show, Finalize)]
impl HeadingNode {
    /// The heading's font family. Just the normal text family if `auto`.
    #[property(referenced)]
    pub const FAMILY: Leveled<Smart<FontFamily>> = Leveled::Value(Smart::Auto);
    /// The color of text in the heading. Just the normal text color if `auto`.
    #[property(referenced)]
    pub const FILL: Leveled<Smart<Paint>> = Leveled::Value(Smart::Auto);
    /// The size of text in the heading.
    #[property(referenced)]
    pub const SIZE: Leveled<TextSize> = Leveled::Mapping(|level| {
        let size = match level.get() {
            1 => 1.4,
            2 => 1.2,
            _ => 1.0,
        };
        TextSize(Em::new(size).into())
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

    /// The spacing above the heading.
    #[property(referenced, shorthand(around))]
    pub const ABOVE: Leveled<Option<BlockSpacing>> = Leveled::Mapping(|level| {
        let ratio = match level.get() {
            1 => 1.5,
            _ => 1.2,
        };
        Some(Ratio::new(ratio).into())
    });
    /// The spacing below the heading.
    #[property(referenced, shorthand(around))]
    pub const BELOW: Leveled<Option<BlockSpacing>> =
        Leveled::Value(Some(Ratio::new(0.55).into()));

    /// Whether the heading appears in the outline.
    pub const OUTLINED: bool = true;
    /// Whether the heading is numbered.
    pub const NUMBERED: bool = true;

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            body: args.expect("body")?,
            level: args.named("level")?.unwrap_or(NonZeroUsize::new(1).unwrap()),
        }
        .pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "level" => Some(Value::Int(self.level.get() as i64)),
            "body" => Some(Value::Content(self.body.clone())),
            _ => None,
        }
    }
}

impl Show for HeadingNode {
    fn unguard_parts(&self, sel: Selector) -> Content {
        Self { body: self.body.unguard(sel), ..*self }.pack()
    }

    fn show(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockNode(self.body.clone()).pack())
    }
}

impl Finalize for HeadingNode {
    fn finalize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
        mut realized: Content,
    ) -> SourceResult<Content> {
        macro_rules! resolve {
            ($key:expr) => {
                styles.get($key).resolve(world, self.level)?
            };
        }

        let mut map = StyleMap::new();
        map.set(TextNode::SIZE, resolve!(Self::SIZE));

        if let Smart::Custom(family) = resolve!(Self::FAMILY) {
            map.set_family(family, styles);
        }

        if let Smart::Custom(fill) = resolve!(Self::FILL) {
            map.set(TextNode::FILL, fill);
        }

        if resolve!(Self::STRONG) {
            realized = realized.strong();
        }

        if resolve!(Self::EMPH) {
            realized = realized.emph();
        }

        if resolve!(Self::UNDERLINE) {
            realized = realized.underlined();
        }

        realized = realized.styled_with_map(map);
        realized = realized.spaced(
            resolve!(Self::ABOVE).resolve(styles),
            resolve!(Self::BELOW).resolve(styles),
        );

        Ok(realized)
    }
}

/// Either the value or a closure mapping to the value.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Leveled<T> {
    /// A bare value.
    Value(T),
    /// A simple mapping from a heading level to a value.
    Mapping(fn(NonZeroUsize) -> T),
    /// A closure mapping from a heading level to a value.
    Func(Func, Span),
}

impl<T: Cast + Clone> Leveled<T> {
    /// Resolve the value based on the level.
    pub fn resolve(
        &self,
        world: Tracked<dyn World>,
        level: NonZeroUsize,
    ) -> SourceResult<T> {
        Ok(match self {
            Self::Value(value) => value.clone(),
            Self::Mapping(mapping) => mapping(level),
            Self::Func(func, span) => {
                let args = Args::new(*span, [Value::Int(level.get() as i64)]);
                func.call_detached(world, args)?.cast().at(*span)?
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
