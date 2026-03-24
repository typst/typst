use std::collections::HashSet;
use std::fmt::Display;
use std::hash::Hash;

use bumpalo::Bump;
use bumpalo::collections::Vec as BumpVec;
use ecow::string::ToEcoString;
use ecow::{EcoString, eco_format};
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};

use crate::css::{Properties, Property};
use crate::{HtmlAttrs, HtmlNode, HtmlTag, attr};

pub struct Stylesheet {
    styles: IndexMap<EcoString, Properties, FxBuildHasher>,
}

impl Stylesheet {
    pub fn new() -> Self {
        Self { styles: IndexMap::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    /// Format the CSS stylesheet.
    pub fn display(&self) -> impl Display {
        typst_utils::display(|f| {
            for (selector, props) in self.styles.iter() {
                writeln!(f, "{selector} {{")?;
                for Property { name, value } in props.iter() {
                    writeln!(f, "  {name}: {value};")?;
                }
                writeln!(f, "}}")?;
            }
            Ok(())
        })
    }
}

/// TODO: Should the hash for [`Properties`] be cached, similar to [`LazyHash`]?
struct Resolver<'a> {
    bump: &'a Bump,
    /// Elements grouped by their CSS properties.
    groups: IndexMap<&'a Properties, Group<'a>, FxBuildHasher>,
    /// Lookup table for groups that contain at least one element with a tag.
    by_tag: FxHashMap<HtmlTag, FxHashSet<GroupId>>,
    /// Lookup table for groups that contain at least one element with a class.
    ///
    /// Simultaneously acts as a string interner for bump allocated class names.
    by_class: FxHashMap<&'a str, FxHashSet<GroupId>>,
}

impl<'a> Resolver<'a> {
    fn new(bump: &'a Bump) -> Self {
        Self {
            bump,
            groups: IndexMap::default(),
            by_tag: FxHashMap::default(),
            by_class: FxHashMap::default(),
        }
    }
}

/// Index into [`Resolver::groups`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct GroupId(u32);

#[derive(Debug, Default)]
struct Group<'a> {
    /// The elements in this group.
    elems: Vec<Elem<'a>>,
}

/// Index into [`Group::elems`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct ElemId(u32);

/// The whole [`HtmlElement`] cannot be borrowed, because that would also
/// include its children.
#[derive(Debug)]
struct Elem<'a> {
    tag: HtmlTag,
    attrs: &'a mut HtmlAttrs,
}

impl<'a> Elem<'a> {
    fn new(tag: HtmlTag, attrs: &'a mut HtmlAttrs) -> Self {
        Self { tag, attrs }
    }
}

/// Resolve a stylesheet from the CSS styles specified for each element.
pub fn resolve_stylesheet(nodes: &mut [HtmlNode]) -> Stylesheet {
    let bump = Bump::new();
    let mut rs = Resolver::new(&bump);

    for node in nodes.iter_mut() {
        visit_node(&mut rs, node);
    }

    identify_groups(&mut rs)
}

/// Build lookup tables to efficiently identify groups of elements sharing the
/// same properties.
fn visit_node<'a>(rs: &mut Resolver<'a>, node: &'a mut HtmlNode) {
    match node {
        HtmlNode::Element(element) => {
            let entry = rs.groups.entry(&element.css);
            let id = GroupId(entry.index() as u32);
            let group = entry.or_default();

            // Tags
            rs.by_tag.entry(element.tag).or_default().insert(id);

            // Classes
            if let Some(class) = element.attrs.get(attr::class) {
                for class in class.split_whitespace() {
                    if let Some(class_groups) = rs.by_class.get_mut(class) {
                        class_groups.insert(id);
                    } else {
                        // Lazily bump allocate the class strings.
                        let class = rs.bump.alloc_str(class);
                        rs.by_class.entry(class).or_default().insert(id);
                    }
                }
            }

            group.elems.push(Elem::new(element.tag, &mut element.attrs));

            for child in element.children.make_mut() {
                visit_node(rs, child);
            }
        }
        HtmlNode::Tag(..) | HtmlNode::Text(..) | HtmlNode::Frame(..) => (),
    }
}

fn identify_groups(rs: &mut Resolver) -> Stylesheet {
    let mut stylesheet = Stylesheet::new();

    let mut class_number = 1;

    for (&props, group) in rs.groups.iter_mut() {
        // The group with an empty set of properties is only included to check
        // for uniqueness of selectors.
        if props.is_empty() {
            continue;
        }

        // TODO: Have some sort of niceness metric at which point we generate
        // our own classes instead of using existing tags and classes. Possibly
        // mixing both.
        let selector = match indentify_group(rs.bump, &rs.by_tag, &rs.by_class, group) {
            Ok(selector) => display_selector_list(selector).to_eco_string(),
            Err(_) => {
                // TODO: Derive better names.
                // Naively generate a custom class name.
                let mut name;
                while {
                    name = eco_format!("typst-{class_number}");
                    class_number += 1;
                    rs.by_class.get(name.as_str()).is_some()
                } {}

                // Add the class attribute.
                for elem in group.elems.iter_mut() {
                    if let Some(classes) = elem.attrs.get_mut(attr::class) {
                        classes.push(' ');
                        classes.push_str(&name);
                    } else {
                        elem.attrs.push_front(attr::class, name.clone());
                    }
                }

                // Make it a class selector
                name.insert(0, '.');

                name
            }
        };
        stylesheet.styles.insert(selector, props.clone());
    }

    stylesheet
}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Selector<S> {
    Type(HtmlTag),
    Class(S),
}

/// Try to uniquely identify a group, ideally using the element tag, or a class
/// that's already present on all elements within it.
fn indentify_group<'a>(
    bump: &'a Bump,
    by_tag: &FxHashMap<HtmlTag, FxHashSet<GroupId>>,
    by_class: &FxHashMap<&'a str, FxHashSet<GroupId>>,
    group: &Group<'a>,
) -> Result<&'a [Selector<&'a str>], &'a [Selector<&'a str>]> {
    // PERF: Consider adding some cutoff optimizations here.

    let mut selectors = IndexMap::<Selector<&'a str>, HashSet<_>>::default();

    // Find class selectors that identify *all* elements within the current
    // group, but no elements from other groups.
    for (i, elem) in group.elems.iter().enumerate() {
        let Some(classes) = elem.attrs.get(attr::class) else { continue };
        for class in classes.split_whitespace() {
            let (class, groups) = by_class.get_key_value(class).unwrap();
            if groups.len() != 1 {
                continue;
            }
            selectors
                .entry(Selector::Class(class))
                .or_default()
                .insert(ElemId(i as u32));
        }
    }

    // Find type (tag) selectors that only identify elements within the current group.
    for (i, elem) in group.elems.iter().enumerate() {
        if by_tag.get(&elem.tag).unwrap().len() != 1 {
            continue;
        }
        selectors
            .entry(Selector::Type(elem.tag))
            .or_default()
            .insert(ElemId(i as u32));
    }

    // Search for a class that all elements have, which is also a unique
    // identifier for this group.
    for (selector, elems) in selectors.iter() {
        if elems.len() == group.elems.len() {
            return Ok(bump.alloc_slice_fill_iter([*selector]));
        }
    }

    // There is no single tag or class that uniquely identifies *all* elements
    // in the group. Try to find an approximately minimal set of tags and
    // classes that fully covers the elements in this group.
    // This is essentially the "Set cover problem":
    // https://en.wikipedia.org/wiki/Set_cover_problem

    let mut num_uncovered = group.elems.len();
    let mut selector_list = BumpVec::new_in(bump);

    // TODO: If we have some sort of niceness score anyway, consider aborting
    // this loop once a threshold is met to not waste too much computation.

    // Build the selector list progressively adding the selector that will cover
    // the most uncovered elements.
    while num_uncovered > 0 && !selectors.is_empty() {
        // Find the selector that covers the most remaining elements.
        let (idx, _) = (selectors.iter().enumerate())
            .max_by_key(|(_, (_, elems))| elems.len())
            .unwrap();
        let (selector, covered) = selectors.shift_remove_index(idx).unwrap();

        selector_list.push(selector);

        // Update the remaining selectors.
        num_uncovered -= covered.len();
        selectors.retain(|_, selector_elems| {
            for elem in covered.iter() {
                selector_elems.remove(elem);
            }
            !selector_elems.is_empty()
        });
    }

    if num_uncovered == 0 {
        Ok(selector_list.into_bump_slice())
    } else {
        Err(selector_list.into_bump_slice())
    }
}

fn display_selector_list(list: &[Selector<&str>]) -> impl Display {
    typst_utils::display(move |f| {
        for (i, selector) in list.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            match selector {
                Selector::Type(tag) => f.write_str(&tag.resolve())?,
                Selector::Class(class) => write!(f, ".{class}")?,
            }
        }
        Ok(())
    })
}
