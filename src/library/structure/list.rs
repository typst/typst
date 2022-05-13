use std::fmt::Write;

use unscanny::Scanner;

use crate::library::layout::{BlockSpacing, GridNode, TrackSizing};
use crate::library::prelude::*;
use crate::library::text::ParNode;
use crate::library::utility::Numbering;

/// An unordered (bulleted) or ordered (numbered) list.
#[derive(Debug, Hash)]
pub struct ListNode<const L: ListKind = UNORDERED> {
    /// Where the list starts.
    pub start: usize,
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// If true, the spacing above the list is leading instead of above spacing.
    pub attached: bool,
    /// The individual bulleted or numbered items.
    pub items: StyleVec<ListItem>,
}

/// An item in a list.
#[derive(Clone, PartialEq, Hash)]
pub struct ListItem {
    /// The kind of item.
    pub kind: ListKind,
    /// The number of the item.
    pub number: Option<usize>,
    /// The node that produces the item's body.
    pub body: Box<Content>,
}

/// An ordered list.
pub type EnumNode = ListNode<ORDERED>;

#[node(showable)]
impl<const L: ListKind> ListNode<L> {
    /// How the list is labelled.
    #[property(referenced)]
    pub const LABEL: Label = Label::Default;
    /// The indentation of each item's label.
    #[property(resolve)]
    pub const INDENT: RawLength = RawLength::zero();
    /// The space between the label and the body of each item.
    #[property(resolve)]
    pub const BODY_INDENT: RawLength = Em::new(0.5).into();

    /// The spacing above the list.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below the list.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing between the items of a wide (non-tight) list.
    #[property(resolve)]
    pub const SPACING: BlockSpacing = Ratio::one().into();

    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::show(Self {
            start: args.named("start")?.unwrap_or(1),
            tight: args.named("tight")?.unwrap_or(true),
            attached: args.named("attached")?.unwrap_or(false),
            items: args
                .all()?
                .into_iter()
                .map(|body| ListItem {
                    kind: L,
                    number: None,
                    body: Box::new(body),
                })
                .collect(),
        }))
    }
}

impl<const L: ListKind> Show for ListNode<L> {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self {
            items: self.items.map(|item| ListItem {
                body: Box::new(item.body.unguard(sel)),
                ..*item
            }),
            ..*self
        }
        .pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "start" => Value::Int(self.start as i64),
            "tight" => Value::Bool(self.tight),
            "attached" => Value::Bool(self.attached),
            "items" => Value::Array(
                self.items
                    .items()
                    .map(|item| Value::Content(item.body.as_ref().clone()))
                    .collect()
            ),
        }
    }

    fn realize(&self, ctx: &mut Context, styles: StyleChain) -> TypResult<Content> {
        let mut cells = vec![];
        let mut number = self.start;

        let label = styles.get(Self::LABEL);

        for (item, map) in self.items.iter() {
            number = item.number.unwrap_or(number);
            cells.push(LayoutNode::default());
            cells
                .push(label.resolve(ctx, L, number)?.styled_with_map(map.clone()).pack());
            cells.push(LayoutNode::default());
            cells.push((*item.body).clone().styled_with_map(map.clone()).pack());
            number += 1;
        }

        let gutter = if self.tight {
            styles.get(ParNode::LEADING)
        } else {
            styles.get(Self::SPACING)
        };

        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::BODY_INDENT);

        Ok(Content::block(GridNode {
            tracks: Spec::with_x(vec![
                TrackSizing::Relative(indent.into()),
                TrackSizing::Auto,
                TrackSizing::Relative(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Spec::with_y(vec![TrackSizing::Relative(gutter.into())]),
            cells,
        }))
    }

    fn finalize(
        &self,
        _: &mut Context,
        styles: StyleChain,
        realized: Content,
    ) -> TypResult<Content> {
        let mut above = styles.get(Self::ABOVE);
        let mut below = styles.get(Self::BELOW);

        if self.attached {
            if above.is_some() {
                above = Some(styles.get(ParNode::LEADING));
            }
            if below.is_some() {
                below = Some(styles.get(ParNode::SPACING));
            }
        }

        Ok(realized.spaced(above, below))
    }
}

impl Debug for ListItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if self.kind == UNORDERED {
            f.write_char('-')?;
        } else {
            if let Some(number) = self.number {
                write!(f, "{}", number)?;
            }
            f.write_char('.')?;
        }
        f.write_char(' ')?;
        self.body.fmt(f)
    }
}

/// How to label a list.
pub type ListKind = usize;

/// Unordered list labelling style.
pub const UNORDERED: ListKind = 0;

/// Ordered list labelling style.
pub const ORDERED: ListKind = 1;

/// How to label a list or enumeration.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Label {
    /// The default labelling.
    Default,
    /// A pattern with prefix, numbering, lower / upper case and suffix.
    Pattern(EcoString, Numbering, bool, EcoString),
    /// Bare content.
    Content(Content),
    /// A closure mapping from an item number to a value.
    Func(Func, Span),
}

impl Label {
    /// Resolve the value based on the level.
    pub fn resolve(
        &self,
        ctx: &mut Context,
        kind: ListKind,
        number: usize,
    ) -> TypResult<Content> {
        Ok(match self {
            Self::Default => match kind {
                UNORDERED => Content::Text('•'.into()),
                ORDERED | _ => Content::Text(format_eco!("{}.", number)),
            },
            Self::Pattern(prefix, numbering, upper, suffix) => {
                let fmt = numbering.apply(number);
                let mid = if *upper { fmt.to_uppercase() } else { fmt.to_lowercase() };
                Content::Text(format_eco!("{}{}{}", prefix, mid, suffix))
            }
            Self::Content(content) => content.clone(),
            Self::Func(func, span) => {
                let args = Args::from_values(*span, [Value::Int(number as i64)]);
                func.call(ctx, args)?.cast().at(*span)?
            }
        })
    }
}

impl Cast<Spanned<Value>> for Label {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(&value.v, Value::Content(_) | Value::Func(_))
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::Str(pattern) => {
                let mut s = Scanner::new(&pattern);
                let mut prefix;
                let numbering = loop {
                    prefix = s.before();
                    match s.eat().map(|c| c.to_ascii_lowercase()) {
                        Some('1') => break Numbering::Arabic,
                        Some('a') => break Numbering::Letter,
                        Some('i') => break Numbering::Roman,
                        Some('*') => break Numbering::Symbol,
                        Some(_) => {}
                        None => Err("invalid pattern")?,
                    }
                };
                let upper = s.scout(-1).map_or(false, char::is_uppercase);
                let suffix = s.after().into();
                Ok(Self::Pattern(prefix.into(), numbering, upper, suffix))
            }
            Value::Content(v) => Ok(Self::Content(v)),
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            _ => Err("expected pattern, content or function")?,
        }
    }
}
