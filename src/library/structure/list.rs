use unscanny::Scanner;

use crate::library::layout::{BlockSpacing, GridNode, TrackSizing};
use crate::library::prelude::*;
use crate::library::text::ParNode;
use crate::library::utility::Numbering;

/// An unordered (bulleted) or ordered (numbered) list.
#[derive(Debug, Hash)]
pub struct ListNode<const L: ListKind = LIST> {
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// If true, the spacing above the list is leading instead of above spacing.
    pub attached: bool,
    /// The individual bulleted or numbered items.
    pub items: StyleVec<ListItem>,
}

/// An ordered list.
pub type EnumNode = ListNode<ENUM>;

/// A description list.
pub type DescNode = ListNode<DESC>;

#[node(showable)]
impl<const L: ListKind> ListNode<L> {
    /// How the list is labelled.
    #[property(referenced)]
    pub const LABEL: Label = Label::Default;
    /// The indentation of each item's label.
    #[property(resolve)]
    pub const INDENT: Length = Length::zero();
    /// The space between the label and the body of each item.
    #[property(resolve)]
    pub const BODY_INDENT: Length = Em::new(match L {
        LIST | ENUM => 0.5,
        DESC | _ => 1.0,
    })
    .into();

    /// The spacing above the list.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below the list.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing between the items of a wide (non-tight) list.
    #[property(resolve)]
    pub const SPACING: BlockSpacing = Ratio::one().into();

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let items = match L {
            LIST => args
                .all()?
                .into_iter()
                .map(|body| ListItem::List(Box::new(body)))
                .collect(),
            ENUM => {
                let mut number: usize = args.named("start")?.unwrap_or(1);
                args.all()?
                    .into_iter()
                    .map(|body| {
                        let item = ListItem::Enum(Some(number), Box::new(body));
                        number += 1;
                        item
                    })
                    .collect()
            }
            DESC | _ => args
                .all()?
                .into_iter()
                .map(|item| ListItem::Desc(Box::new(item)))
                .collect(),
        };

        Ok(Content::show(Self {
            tight: args.named("tight")?.unwrap_or(true),
            attached: args.named("attached")?.unwrap_or(false),
            items,
        }))
    }
}

impl<const L: ListKind> Show for ListNode<L> {
    fn unguard(&self, sel: Selector) -> ShowNode {
        Self {
            items: self.items.map(|item| item.unguard(sel)),
            ..*self
        }
        .pack()
    }

    fn encode(&self, _: StyleChain) -> Dict {
        dict! {
            "tight" => Value::Bool(self.tight),
            "attached" => Value::Bool(self.attached),
            "items" => Value::Array(
                self.items
                    .items()
                    .map(|item| item.encode())
                    .collect()
            ),
        }
    }

    fn realize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let mut cells = vec![];
        let mut number = 1;

        let label = styles.get(Self::LABEL);
        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::BODY_INDENT);
        let gutter = if self.tight {
            styles.get(ParNode::LEADING)
        } else {
            styles.get(Self::SPACING)
        };

        for (item, map) in self.items.iter() {
            if let &ListItem::Enum(Some(n), _) = item {
                number = n;
            }

            cells.push(LayoutNode::default());

            let label = if L == LIST || L == ENUM {
                label.resolve(world, L, number)?.styled_with_map(map.clone()).pack()
            } else {
                LayoutNode::default()
            };

            cells.push(label);
            cells.push(LayoutNode::default());

            let body = match &item {
                ListItem::List(body) => body.as_ref().clone(),
                ListItem::Enum(_, body) => body.as_ref().clone(),
                ListItem::Desc(item) => Content::sequence(vec![
                    Content::Horizontal {
                        amount: (-body_indent).into(),
                        weak: false,
                    },
                    (item.term.clone() + Content::Text(':'.into())).strong(),
                    Content::Space,
                    item.body.clone(),
                ]),
            };

            cells.push(body.styled_with_map(map.clone()).pack());
            number += 1;
        }

        Ok(Content::block(GridNode {
            tracks: Axes::with_x(vec![
                TrackSizing::Relative(indent.into()),
                TrackSizing::Auto,
                TrackSizing::Relative(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Axes::with_y(vec![TrackSizing::Relative(gutter.into())]),
            cells,
        }))
    }

    fn finalize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
        realized: Content,
    ) -> SourceResult<Content> {
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

/// An item in a list.
#[derive(Clone, PartialEq, Hash)]
pub enum ListItem {
    /// An item of an unordered list.
    List(Box<Content>),
    /// An item of an ordered list.
    Enum(Option<usize>, Box<Content>),
    /// An item of a description list.
    Desc(Box<DescItem>),
}

impl ListItem {
    /// What kind of item this is.
    pub fn kind(&self) -> ListKind {
        match self {
            Self::List(_) => LIST,
            Self::Enum { .. } => ENUM,
            Self::Desc { .. } => DESC,
        }
    }

    fn unguard(&self, sel: Selector) -> Self {
        match self {
            Self::List(body) => Self::List(Box::new(body.unguard(sel))),
            Self::Enum(number, body) => Self::Enum(*number, Box::new(body.unguard(sel))),
            Self::Desc(item) => Self::Desc(Box::new(DescItem {
                term: item.term.unguard(sel),
                body: item.body.unguard(sel),
            })),
        }
    }

    /// Encode the item into a value.
    fn encode(&self) -> Value {
        match self {
            Self::List(body) => Value::Content(body.as_ref().clone()),
            Self::Enum(number, body) => Value::Dict(dict! {
                "number" => match *number {
                    Some(n) => Value::Int(n as i64),
                    None => Value::None,
                },
                "body" => Value::Content(body.as_ref().clone()),
            }),
            Self::Desc(item) => Value::Dict(dict! {
                "term" => Value::Content(item.term.clone()),
                "body" => Value::Content(item.body.clone()),
            }),
        }
    }
}

impl Debug for ListItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::List(body) => write!(f, "- {body:?}"),
            Self::Enum(number, body) => match number {
                Some(n) => write!(f, "{n}. {body:?}"),
                None => write!(f, "+ {body:?}"),
            },
            Self::Desc(item) => item.fmt(f),
        }
    }
}

/// A description list item.
#[derive(Clone, PartialEq, Hash)]
pub struct DescItem {
    /// The term described by the list item.
    pub term: Content,
    /// The description of the term.
    pub body: Content,
}

castable! {
    DescItem,
    Expected: "dictionary with `term` and `body` keys",
    Value::Dict(dict) => {
        let term: Content = dict.get("term")?.clone().cast()?;
        let body: Content = dict.get("body")?.clone().cast()?;
        Self { term, body }
    },
}

impl Debug for DescItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "/ {:?}: {:?}", self.term, self.body)
    }
}

/// How to label a list.
pub type ListKind = usize;

/// An unordered list.
pub const LIST: ListKind = 0;

/// An ordered list.
pub const ENUM: ListKind = 1;

/// A description list.
pub const DESC: ListKind = 2;

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
        world: Tracked<dyn World>,
        kind: ListKind,
        number: usize,
    ) -> SourceResult<Content> {
        Ok(match self {
            Self::Default => match kind {
                LIST => Content::Text('•'.into()),
                ENUM => Content::Text(format_eco!("{}.", number)),
                DESC | _ => panic!("description lists don't have a label"),
            },
            Self::Pattern(prefix, numbering, upper, suffix) => {
                let fmt = numbering.apply(number);
                let mid = if *upper { fmt.to_uppercase() } else { fmt.to_lowercase() };
                Content::Text(format_eco!("{}{}{}", prefix, mid, suffix))
            }
            Self::Content(content) => content.clone(),
            Self::Func(func, span) => {
                let args = Args::new(*span, [Value::Int(number as i64)]);
                func.call_detached(world, args)?.display(world)
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
            Value::None => Ok(Self::Content(Content::Empty)),
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
            v => Err(format!(
                "expected string, content or function, found {}",
                v.type_name(),
            )),
        }
    }
}
