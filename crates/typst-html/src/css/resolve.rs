//! This modules tries to resolve a [`Stylesheet`] from the CSS properties that
//! are specified for each element. Existing classes, ARIA roles, and tags are
//! used where possible, otherwise custom classes are generated and assigned to
//! elements.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::hash::Hash;

use bumpalo::Bump;
use bumpalo::collections::{CollectIn, Vec as BumpVec};
use ecow::{EcoString, eco_format, eco_vec};
use rustc_hash::{FxHashMap, FxHashSet};
use typst_library::foundations::BundlePath;
use typst_syntax::{Span, Spanned, VirtualPath};

use crate::css::{FilteredProperties, Property};
use crate::{HtmlElement, HtmlNode, HtmlTag, attr};
use crate::{mathml, tag};

/// Turns CSS properties on all elements in the DOM into inline `style`
/// attributes.
///
/// This by itself would not need a separate pass over the DOM, but it will be
/// supplanted by more advanced CSS handling, so it makes sense to already
/// organize the code like this.
pub fn write_inline_styles(root: &mut HtmlElement) {
    write_elem_inline_styles(root);
}

fn write_elem_inline_styles(elem: &mut HtmlElement) {
    if !elem.css.is_empty() {
        // TODO: Use to_eco_string once merged:
        // https://github.com/typst/ecow/pull/60
        let mut generated = eco_format!("{}", elem.css.to_inline());
        if let Some(style) = elem.attrs.get_mut(attr::style) {
            if !style.is_empty() {
                generated.push_str("; ");
            }
            generated.push_str(style);
            *style = generated;
        } else {
            elem.attrs.push(attr::style, generated);
        }
    }

    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(elem) = child {
            write_elem_inline_styles(elem);
        }
    }
}

/// Find selector candidates for a CSS stylesheet. This is the first and
/// incremental part of automatic stylesheet generation.
pub fn find_selector_candidates(root: &HtmlElement) -> SelectorCandidates {
    let mut ctx = SelectorCandidates::default();
    find_element_candidates(&mut ctx, root);
    ctx
}

#[derive(Debug, Clone)]
pub struct ExternalCss {
    /// The location of the external stylesheet.
    pub path: Spanned<BundlePath>,
    /// Data used to generate the external stylesheet.
    pub data: StylesheetData,
}

impl ExternalCss {
    /// Create new external stylesheet data.
    pub fn new(path: Spanned<BundlePath>, data: StylesheetData) -> Self {
        Self { path, data }
    }
}

/// Data used to generate a stylesheet.
#[derive(Debug, Default, Clone)]
pub struct StylesheetData {
    /// A list of selector candidates that can be used to target CSS property
    /// value pairs.
    pub candidates: SelectorCandidates,
    /// Whether there is any MathML in form of `<math>` elements.
    /// The current automatic stylesheet generation system isn't capable of
    /// producing an appropriate stylesheet, so a manually written one is
    /// included instead.
    pub has_math: bool,
}

impl StylesheetData {
    /// Create new stylesheet data.
    pub fn new(candidates: SelectorCandidates, has_math: bool) -> Self {
        Self { candidates, has_math }
    }

    /// Merge stylesheet data from another document to produce a stylesheet that
    /// covers both documents.
    pub fn merge(&mut self, other: &StylesheetData) {
        self.has_math |= other.has_math;
        self.candidates.merge(&other.candidates);
    }
}

/// A lookup table from selector candidates to properties that can be targeted
/// by them.
#[derive(Debug, Default, Clone)]
pub struct SelectorCandidates {
    /// Invariant: The property list is always sorted.
    candidates: FxHashMap<SimpleSelector, FilteredProperties>,
}

impl SelectorCandidates {
    /// Merge selector candidates from another document to produce a stylesheet
    /// that covers both documents.
    pub fn merge(&mut self, other: &Self) {
        #[expect(clippy::iter_over_hash_type)]
        for (s, p) in &other.candidates {
            insert_selector_props(&mut self.candidates, Cow::Borrowed(s), p);
        }
    }

    fn insert(&mut self, selector: SimpleSelector, props: &FilteredProperties) {
        insert_selector_props(&mut self.candidates, Cow::Owned(selector), props);
    }

    /// Whether the property can be targeted by the selector.
    fn can_target(&self, selector: &SimpleSelector, prop: &Property) -> bool {
        let Some(props) = self.candidates.get(selector) else { return false };
        props.binary_search(prop).is_ok()
    }
}

fn find_element_candidates(ctx: &mut SelectorCandidates, elem: &HtmlElement) {
    for selector in elem_selectors(elem) {
        ctx.insert(selector, &elem.css);
    }

    for node in &elem.children {
        if let HtmlNode::Element(child) = node {
            find_element_candidates(ctx, child);
        }
    }
}

fn elem_selectors(elem: &HtmlElement) -> impl Iterator<Item = SimpleSelector> {
    let classes = (elem.attrs.get(attr::class).into_iter())
        .flat_map(|classes| classes.split_whitespace());
    let role = elem.attrs.get(attr::role);

    // Order by priority: classes, role, type
    classes
        .map(|class| SimpleSelector::Class(EcoString::from(class)))
        .chain(role.map(|role| SimpleSelector::Role(role.clone())))
        .chain([SimpleSelector::ty(elem.tag)])
}

pub fn insert_external_stylesheet_link(root: &mut HtmlElement, path: &VirtualPath) {
    let head = head_mut(root);
    head.children.push(
        HtmlElement::new(tag::link)
            .with_attr(attr::rel, "stylesheet")
            .with_attr(attr::href, path.get_with_slash())
            .into(),
    );
}

pub fn insert_embedded_stylesheet(root: &mut HtmlElement, stylesheet: EcoString) {
    let head = head_mut(root);
    head.children.push(
        HtmlElement::new(tag::style)
            .with_children(eco_vec![HtmlNode::Text(stylesheet, Span::detached(),)])
            .into(),
    );
}

fn head_mut(root: &mut HtmlElement) -> &mut HtmlElement {
    let head = root.children.make_mut().iter_mut().find_map(|node| match node {
        HtmlNode::Element(elem) if elem.tag == tag::head => Some(elem),
        _ => None,
    });

    // TODO: this becomes an error when html fragments are supported
    head.expect("head to be present in document output")
}

pub fn resolve_stylesheet(
    root_elems: &mut [&mut HtmlElement],
    data: &StylesheetData,
) -> EcoString {
    let bump = Bump::new();
    let styles = resolve_styles(&bump, root_elems, &data.candidates);
    let mut stylesheet = eco_format!("{}", styles.display());
    if data.has_math {
        stylesheet.push_str(&mathml::EQUATION_CSS_STYLES);
    }
    stylesheet
}

fn resolve_styles<'a>(
    bump: &'a Bump,
    root_elems: &mut [&mut HtmlElement],
    candidates: &SelectorCandidates,
) -> Stylesheet<'a> {
    let mut rs = Resolver::new(bump);
    for elem in root_elems {
        resolve_elem_styles(&mut rs, elem, candidates);
    }

    let mut stylesheet = Stylesheet::default();
    #[expect(clippy::iter_over_hash_type)]
    for (Property { name, value }, selectors) in rs.properties {
        let mut selector_list = BumpVec::new_in(rs.bump);

        // Sort the selectors.
        selector_list.extend(selectors.list.into_iter().map(OrderedSelector));
        selector_list.sort();

        // Add the custom class last.
        selector_list
            .extend(selectors.custom.map(SimpleSelector::Class).map(OrderedSelector));

        let list = selector_list.into_bump_slice();
        stylesheet.styles.entry(list).or_default().push(name, value);
    }

    stylesheet
}

struct Resolver<'a> {
    bump: &'a Bump,
    properties: FxHashMap<Property, SelectorList>,
    class_number: usize,
}

impl<'a> Resolver<'a> {
    fn new(bump: &'a Bump) -> Self {
        Self {
            bump,
            properties: FxHashMap::default(),
            class_number: 0,
        }
    }

    fn generate_custom_class(&mut self, prop: Property) -> &EcoString {
        let props = self.properties.entry(prop).or_default();
        props.custom.get_or_insert_with(|| {
            self.class_number += 1;
            eco_format!("typst{}", self.class_number)
        })
    }
}

#[derive(Default)]
struct SelectorList {
    list: FxHashSet<SimpleSelector>,
    custom: Option<EcoString>,
}

fn resolve_elem_styles(
    rs: &mut Resolver,
    elem: &mut HtmlElement,
    candidates: &SelectorCandidates,
) {
    let elem_selectors = elem_selectors(elem).collect_in::<BumpVec<_>>(rs.bump);
    'properties: for prop in elem.css.iter() {
        if let Some(prop_selectors) = rs.properties.get_mut(prop)
            && elem_selectors.iter().any(|s| prop_selectors.list.contains(s))
        {
            // A selector that targets this element is already present in the
            // selector list: nothing to do.
            continue 'properties;
        }

        // Try to find a selector that's already on the element.
        for selector in &elem_selectors {
            if candidates.can_target(selector, prop) {
                rs.properties
                    .entry(prop.clone())
                    .or_default()
                    .list
                    .insert(selector.clone());
                continue 'properties;
            }
        }

        // Generate a custom class an add it to the element.
        let custom = rs.generate_custom_class(prop.clone());
        if let Some(classes) = elem.attrs.get_mut(attr::class) {
            classes.push(' ');
            classes.push_str(custom);
        } else {
            elem.attrs.push(attr::class, custom);
        }
    }
    drop(elem_selectors);

    for node in elem.children.make_mut() {
        if let HtmlNode::Element(child) = node {
            resolve_elem_styles(rs, child, candidates);
        }
    }
}

/// A resolved stylesheet that maps from serialized selector lists to CSS
/// properties.
#[derive(Default)]
struct Stylesheet<'a> {
    styles: BTreeMap<&'a [OrderedSelector], FilteredProperties>,
}

impl Stylesheet<'_> {
    /// Format the CSS stylesheet.
    pub fn display(&self) -> impl Display {
        typst_utils::display(|f| {
            for (list, props) in &self.styles {
                for (i, OrderedSelector(selector)) in list.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{selector}")?;
                }
                writeln!(f, " {{")?;
                for Property { name, value } in props.iter() {
                    writeln!(f, "  {name}: {value};")?;
                }
                writeln!(f, "}}")?;
            }
            Ok(())
        })
    }
}

#[derive(Eq, PartialEq)]
struct OrderedSelector(SimpleSelector);

impl Ord for OrderedSelector {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.alphanumeric_cmp(&other.0)
    }
}

impl PartialOrd for OrderedSelector {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Inserts the properties belonging to the selector into the lookup table.
///
/// If the selector is new, the whole set of properties can be targeted by it.
/// If the selector has been found before the set of properties that can be
/// targeted by it is the intersection of the two property lists.
fn insert_selector_props(
    selectors: &mut FxHashMap<SimpleSelector, FilteredProperties>,
    selector: Cow<SimpleSelector>,
    properties: &FilteredProperties,
) {
    let Some(existing) = selectors.get_mut(&selector) else {
        // The whole set of properties can be targeted.
        selectors.insert(selector.into_owned(), properties.clone());
        return;
    };

    // The intersection with an empty set is always empty.
    if existing.is_empty() {
        return;
    }

    // Iterate the two sorted lists in tandem.
    let mut new_iter = properties.iter().peekable();
    existing.retain(|target| {
        // Skip all properties smaller than the target one.
        while new_iter.next_if(|&new| new < target).is_some() {}

        // Retain the target if it's also inside the element's groups.
        new_iter.next_if(|&new| new == target).is_some()
    });
}

/// A CSS selector.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SimpleSelector {
    /// A HTML tag.
    /// E.g. `li`
    Type(HtmlTagKey),
    /// An ARIA role.
    /// E.g. `[role=tab]`
    Role(EcoString),
    /// A CSS class.
    /// E.g. `.class`
    Class(EcoString),
}

impl SimpleSelector {
    fn ty(tag: HtmlTag) -> Self {
        Self::Type(HtmlTagKey(tag))
    }

    fn alphanumeric_cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Type(a), Self::Type(b)) => {
                // The `Ord` implementation of `HtmlTagKey` doesn't sort by
                // string contents for performance reasons. For the CSS
                // generation it's preferable to sort alphabetically even though
                // it will be slower.
                a.0.resolve().as_str().cmp(b.0.resolve().as_str())
            }
            _ => self.cmp(other),
        }
    }
}

impl Display for SimpleSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleSelector::Type(tag) => f.write_str(&tag.0.resolve()),
            SimpleSelector::Role(role) => write!(f, "[role={role}]"),
            SimpleSelector::Class(class) => write!(f, ".{class}"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct HtmlTagKey(pub HtmlTag);

impl Ord for HtmlTagKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Use the opaque numeric `PicoStr` key for ordering HTML tags. We don't
        // really care about what ordering, just that there is *some* consistent
        // ordering of selectors.
        let a = self.0.into_inner().opaque_key();
        let b = other.0.into_inner().opaque_key();
        a.cmp(&b)
    }
}

impl PartialOrd for HtmlTagKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
