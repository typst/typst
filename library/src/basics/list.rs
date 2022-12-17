use crate::compute::NumberingPattern;
use crate::layout::{BlockNode, GridNode, HNode, ParNode, Spacing, TrackSizing};
use crate::prelude::*;
use crate::text::{SpaceNode, TextNode};

/// An unordered (bulleted) or ordered (numbered) list.
///
/// # Parameters
/// - items: Content (positional, variadic)
///   The contents of the list items.
/// - start: NonZeroUsize (named)
///   Which number to start the enumeration with.
/// - tight: bool (named)
///   Makes the list more compact, if enabled. This looks better if the items
///   fit into a single line each.
///
/// # Tags
/// - basics
#[func]
#[capable(Layout)]
#[derive(Debug, Hash)]
pub struct ListNode<const L: ListKind = LIST> {
    /// If true, the items are separated by leading instead of list spacing.
    pub tight: bool,
    /// The individual bulleted or numbered items.
    pub items: StyleVec<ListItem>,
}

/// An ordered list.
pub type EnumNode = ListNode<ENUM>;

/// A description list.
pub type DescNode = ListNode<DESC>;

#[node]
impl<const L: ListKind> ListNode<L> {
    /// How the list is labelled.
    #[property(referenced)]
    pub const LABEL: ListLabel = ListLabel::Default;
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
    /// The spacing between the items of a wide (non-tight) list.
    pub const SPACING: Smart<Spacing> = Smart::Auto;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let items = match L {
            LIST => args
                .all()?
                .into_iter()
                .map(|body| ListItem::List(Box::new(body)))
                .collect(),
            ENUM => {
                let mut number: NonZeroUsize =
                    args.named("start")?.unwrap_or(NonZeroUsize::new(1).unwrap());
                args.all()?
                    .into_iter()
                    .map(|body| {
                        let item = ListItem::Enum(Some(number), Box::new(body));
                        number = number.saturating_add(1);
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

        Ok(Self { tight: args.named("tight")?.unwrap_or(true), items }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "tight" => Some(Value::Bool(self.tight)),
            "items" => {
                Some(Value::Array(self.items.items().map(|item| item.encode()).collect()))
            }
            _ => None,
        }
    }
}

impl<const L: ListKind> Layout for ListNode<L> {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut cells = vec![];
        let mut number = NonZeroUsize::new(1).unwrap();

        let label = styles.get(Self::LABEL);
        let indent = styles.get(Self::INDENT);
        let body_indent = styles.get(Self::BODY_INDENT);
        let gutter = if self.tight {
            styles.get(ParNode::LEADING).into()
        } else {
            styles
                .get(Self::SPACING)
                .unwrap_or_else(|| styles.get(BlockNode::BELOW).amount)
        };

        for (item, map) in self.items.iter() {
            if let &ListItem::Enum(Some(n), _) = item {
                number = n;
            }

            cells.push(Content::empty());

            let label = if L == LIST || L == ENUM {
                label.resolve(vt, L, number)?.styled_with_map(map.clone())
            } else {
                Content::empty()
            };

            cells.push(label);
            cells.push(Content::empty());

            let body = match &item {
                ListItem::List(body) => body.as_ref().clone(),
                ListItem::Enum(_, body) => body.as_ref().clone(),
                ListItem::Desc(item) => Content::sequence(vec![
                    HNode { amount: (-body_indent).into(), weak: false }.pack(),
                    (item.term.clone() + TextNode::packed(':')).strong(),
                    SpaceNode.pack(),
                    item.body.clone(),
                ]),
            };

            cells.push(body.styled_with_map(map.clone()));
            number = number.saturating_add(1);
        }

        GridNode {
            tracks: Axes::with_x(vec![
                TrackSizing::Relative(indent.into()),
                TrackSizing::Auto,
                TrackSizing::Relative(body_indent.into()),
                TrackSizing::Auto,
            ]),
            gutter: Axes::with_y(vec![gutter.into()]),
            cells,
        }
        .layout(vt, styles, regions)
    }
}

/// An item in a list.
#[capable]
#[derive(Debug, Clone, Hash)]
pub enum ListItem {
    /// An item of an unordered list.
    List(Box<Content>),
    /// An item of an ordered list.
    Enum(Option<NonZeroUsize>, Box<Content>),
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

    /// Encode the item into a value.
    fn encode(&self) -> Value {
        match self {
            Self::List(body) => Value::Content(body.as_ref().clone()),
            Self::Enum(number, body) => Value::Dict(dict! {
                "number" => match *number {
                    Some(n) => Value::Int(n.get() as i64),
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

#[node]
impl ListItem {}

/// A description list item.
#[derive(Debug, Clone, Hash)]
pub struct DescItem {
    /// The term described by the list item.
    pub term: Content,
    /// The description of the term.
    pub body: Content,
}

castable! {
    DescItem,
    mut dict: Dict => {
        let term: Content = dict.take("term")?.cast()?;
        let body: Content = dict.take("body")?.cast()?;
        dict.finish(&["term", "body"])?;
        Self { term, body }
    },
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
#[derive(Debug, Clone, Hash)]
pub enum ListLabel {
    /// The default labelling.
    Default,
    /// A pattern with prefix, numbering, lower / upper case and suffix.
    Pattern(NumberingPattern),
    /// Bare content.
    Content(Content),
    /// A closure mapping from an item number to a value.
    Func(Func, Span),
}

impl ListLabel {
    /// Resolve the label based on the level.
    pub fn resolve(
        &self,
        vt: &Vt,
        kind: ListKind,
        number: NonZeroUsize,
    ) -> SourceResult<Content> {
        Ok(match self {
            Self::Default => match kind {
                LIST => TextNode::packed('•'),
                ENUM => TextNode::packed(format_eco!("{}.", number)),
                DESC | _ => panic!("description lists don't have a label"),
            },
            Self::Pattern(pattern) => TextNode::packed(pattern.apply(&[number])),
            Self::Content(content) => content.clone(),
            Self::Func(func, span) => {
                let args = Args::new(*span, [Value::Int(number.get() as i64)]);
                func.call_detached(vt.world(), args)?.display()
            }
        })
    }
}

impl Cast<Spanned<Value>> for ListLabel {
    fn is(value: &Spanned<Value>) -> bool {
        matches!(
            &value.v,
            Value::None | Value::Str(_) | Value::Content(_) | Value::Func(_)
        )
    }

    fn cast(value: Spanned<Value>) -> StrResult<Self> {
        match value.v {
            Value::None => Ok(Self::Content(Content::empty())),
            Value::Str(v) => Ok(Self::Pattern(v.parse()?)),
            Value::Content(v) => Ok(Self::Content(v)),
            Value::Func(v) => Ok(Self::Func(v, value.span)),
            v => Self::error(v),
        }
    }

    fn describe() -> CastInfo {
        CastInfo::Union(vec![
            CastInfo::Type("string"),
            CastInfo::Type("content"),
            CastInfo::Type("function"),
        ])
    }
}
