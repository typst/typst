use std::mem;

use comemo::Tracked;
use typed_arena::Arena;

use super::collapse::CollapsingBuilder;
use super::{
    Barrier, Content, Interruption, Layout, Level, Node, SequenceNode, Show, StyleChain,
    StyleEntry, StyleMap, StyleVecBuilder, StyledNode, Target,
};
use crate::diag::SourceResult;
use crate::geom::Numeric;
use crate::library::layout::{
    ColbreakNode, FlowChild, FlowNode, HNode, PageNode, PagebreakNode, PlaceNode, VNode,
};
use crate::library::structure::{DocNode, ListItem, ListNode, DESC, ENUM, LIST};
use crate::library::text::{
    LinebreakNode, ParChild, ParNode, ParbreakNode, SmartQuoteNode, SpaceNode, TextNode,
};
use crate::World;

/// Builds a document or a flow node from content.
pub(super) struct Builder<'a> {
    /// The core context.
    world: Tracked<'a, dyn World>,
    /// Scratch arenas for building.
    scratch: &'a Scratch<'a>,
    /// The current document building state.
    doc: Option<DocBuilder<'a>>,
    /// The current flow building state.
    flow: FlowBuilder<'a>,
    /// The current paragraph building state.
    par: ParBuilder<'a>,
    /// The current list building state.
    list: ListBuilder<'a>,
}

/// Temporary storage arenas for building.
#[derive(Default)]
pub(super) struct Scratch<'a> {
    /// An arena where intermediate style chains are stored.
    styles: Arena<StyleChain<'a>>,
    /// An arena where intermediate content resulting from show rules is stored.
    templates: Arena<Content>,
}

impl<'a> Builder<'a> {
    pub fn new(
        world: Tracked<'a, dyn World>,
        scratch: &'a Scratch<'a>,
        top: bool,
    ) -> Self {
        Self {
            world,
            scratch,
            doc: top.then(|| DocBuilder::default()),
            flow: FlowBuilder::default(),
            par: ParBuilder::default(),
            list: ListBuilder::default(),
        }
    }

    pub fn into_doc(
        mut self,
        styles: StyleChain<'a>,
    ) -> SourceResult<(DocNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Page, styles, true)?;
        let (pages, shared) = self.doc.unwrap().pages.finish();
        Ok((DocNode(pages), shared))
    }

    pub fn into_flow(
        mut self,
        styles: StyleChain<'a>,
    ) -> SourceResult<(FlowNode, StyleChain<'a>)> {
        self.interrupt(Interruption::Par, styles, false)?;
        let (children, shared) = self.flow.0.finish();
        Ok((FlowNode(children), shared))
    }

    pub fn accept(
        &mut self,
        content: &'a Content,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        if let Some(node) = content.downcast::<TextNode>() {
            if let Some(realized) = styles.apply(self.world, Target::Text(&node.0))? {
                let stored = self.scratch.templates.alloc(realized);
                return self.accept(stored, styles);
            }
        } else if let Some(styled) = content.downcast::<StyledNode>() {
            return self.styled(styled, styles);
        } else if let Some(seq) = content.downcast::<SequenceNode>() {
            return self.sequence(seq, styles);
        } else if content.has::<dyn Show>() {
            if self.show(&content, styles)? {
                return Ok(());
            }
        }

        if self.list.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::List, styles, false)?;

        if content.is::<ListItem>() {
            self.list.accept(content, styles);
            return Ok(());
        }

        if self.par.accept(content, styles) {
            return Ok(());
        }

        self.interrupt(Interruption::Par, styles, false)?;

        if self.flow.accept(content, styles) {
            return Ok(());
        }

        let keep = content.downcast::<PagebreakNode>().map_or(false, |node| !node.weak);
        self.interrupt(Interruption::Page, styles, keep)?;

        if let Some(doc) = &mut self.doc {
            doc.accept(content, styles);
        }

        // We might want to issue a warning or error for content that wasn't
        // handled (e.g. a pagebreak in a flow building process). However, we
        // don't have the spans here at the moment.
        Ok(())
    }

    fn show(&mut self, node: &'a Content, styles: StyleChain<'a>) -> SourceResult<bool> {
        if let Some(mut realized) = styles.apply(self.world, Target::Node(node))? {
            let mut map = StyleMap::new();
            let barrier = Barrier::new(node.id());
            map.push(StyleEntry::Barrier(barrier));
            map.push(StyleEntry::Barrier(barrier));
            realized = realized.styled_with_map(map);
            let stored = self.scratch.templates.alloc(realized);
            self.accept(stored, styles)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn styled(
        &mut self,
        styled: &'a StyledNode,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        let stored = self.scratch.styles.alloc(styles);
        let styles = styled.map.chain(stored);
        let intr = styled.map.interruption();

        if let Some(intr) = intr {
            self.interrupt(intr, styles, false)?;
        }

        self.accept(&styled.sub, styles)?;

        if let Some(intr) = intr {
            self.interrupt(intr, styles, true)?;
        }

        Ok(())
    }

    fn interrupt(
        &mut self,
        intr: Interruption,
        styles: StyleChain<'a>,
        keep: bool,
    ) -> SourceResult<()> {
        if intr >= Interruption::List && !self.list.is_empty() {
            mem::take(&mut self.list).finish(self)?;
        }

        if intr >= Interruption::Par {
            if !self.par.is_empty() {
                mem::take(&mut self.par).finish(self);
            }
        }

        if intr >= Interruption::Page {
            if let Some(doc) = &mut self.doc {
                if !self.flow.is_empty() || (doc.keep_next && keep) {
                    mem::take(&mut self.flow).finish(doc, styles);
                }
                doc.keep_next = !keep;
            }
        }

        Ok(())
    }

    fn sequence(
        &mut self,
        seq: &'a SequenceNode,
        styles: StyleChain<'a>,
    ) -> SourceResult<()> {
        for content in &seq.0 {
            self.accept(content, styles)?;
        }
        Ok(())
    }
}

/// Accepts pagebreaks and pages.
struct DocBuilder<'a> {
    /// The page runs built so far.
    pages: StyleVecBuilder<'a, PageNode>,
    /// Whether to keep a following page even if it is empty.
    keep_next: bool,
}

impl<'a> DocBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) {
        if let Some(pagebreak) = content.downcast::<PagebreakNode>() {
            self.keep_next = !pagebreak.weak;
        }

        if let Some(page) = content.downcast::<PageNode>() {
            self.pages.push(page.clone(), styles);
            self.keep_next = false;
        }
    }
}

impl Default for DocBuilder<'_> {
    fn default() -> Self {
        Self {
            pages: StyleVecBuilder::new(),
            keep_next: true,
        }
    }
}

/// Accepts flow content.
#[derive(Default)]
struct FlowBuilder<'a>(CollapsingBuilder<'a, FlowChild>);

impl<'a> FlowBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        // Weak flow elements:
        // Weakness | Element
        //    0     | weak colbreak
        //    1     | weak fractional spacing
        //    2     | weak spacing
        //    3     | generated weak spacing
        //    4     | generated weak fractional spacing
        //    5     | par spacing

        if let Some(_) = content.downcast::<ParbreakNode>() {
            /* Nothing to do */
        } else if let Some(colbreak) = content.downcast::<ColbreakNode>() {
            if colbreak.weak {
                self.0.weak(FlowChild::Colbreak, styles, 0);
            } else {
                self.0.destructive(FlowChild::Colbreak, styles);
            }
        } else if let Some(vertical) = content.downcast::<VNode>() {
            let child = FlowChild::Spacing(vertical.amount);
            let frac = vertical.amount.is_fractional();
            if vertical.weak {
                let weakness = 1 + u8::from(frac) + 2 * u8::from(vertical.generated);
                self.0.weak(child, styles, weakness);
            } else if frac {
                self.0.destructive(child, styles);
            } else {
                self.0.ignorant(child, styles);
            }
        } else if content.has::<dyn Layout>() {
            let child = FlowChild::Node(content.clone());
            if content.is::<PlaceNode>() {
                self.0.ignorant(child, styles);
            } else {
                self.0.supportive(child, styles);
            }
        } else {
            return false;
        }

        true
    }

    fn par(&mut self, par: ParNode, styles: StyleChain<'a>, indent: bool) {
        let amount = if indent && !styles.get(ParNode::SPACING_AND_INDENT) {
            styles.get(ParNode::LEADING).into()
        } else {
            styles.get(ParNode::SPACING).into()
        };

        self.0.weak(FlowChild::Spacing(amount), styles, 5);
        self.0.supportive(FlowChild::Node(par.pack()), styles);
        self.0.weak(FlowChild::Spacing(amount), styles, 5);
    }

    fn finish(self, doc: &mut DocBuilder<'a>, styles: StyleChain<'a>) {
        let (flow, shared) = self.0.finish();
        let styles = if flow.is_empty() { styles } else { shared };
        let node = PageNode(FlowNode(flow).pack());
        doc.pages.push(node, styles);
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Accepts paragraph content.
#[derive(Default)]
struct ParBuilder<'a>(CollapsingBuilder<'a, ParChild>);

impl<'a> ParBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        // Weak par elements:
        // Weakness | Element
        //    0     | weak fractional spacing
        //    1     | weak spacing
        //    2     | space

        if content.is::<SpaceNode>() {
            self.0.weak(ParChild::Text(' '.into()), styles, 2);
        } else if let Some(linebreak) = content.downcast::<LinebreakNode>() {
            let c = if linebreak.justify { '\u{2028}' } else { '\n' };
            self.0.destructive(ParChild::Text(c.into()), styles);
        } else if let Some(horizontal) = content.downcast::<HNode>() {
            let child = ParChild::Spacing(horizontal.amount);
            let frac = horizontal.amount.is_fractional();
            if horizontal.weak {
                let weakness = u8::from(!frac);
                self.0.weak(child, styles, weakness);
            } else if frac {
                self.0.destructive(child, styles);
            } else {
                self.0.ignorant(child, styles);
            }
        } else if let Some(quote) = content.downcast::<SmartQuoteNode>() {
            self.0.supportive(ParChild::Quote { double: quote.double }, styles);
        } else if let Some(node) = content.downcast::<TextNode>() {
            self.0.supportive(ParChild::Text(node.0.clone()), styles);
        } else if let Some(node) = content.to::<dyn Layout>() {
            if node.level() == Level::Inline {
                self.0.supportive(ParChild::Node(content.clone()), styles);
            } else {
                return false;
            }
        } else {
            return false;
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a>) {
        let (mut children, shared) = self.0.finish();
        if children.is_empty() {
            return;
        }

        // Paragraph indent should only apply if the paragraph starts with
        // text and follows directly after another paragraph.
        let indent = shared.get(ParNode::INDENT);
        if !indent.is_zero()
            && children
                .items()
                .find_map(|child| match child {
                    ParChild::Spacing(_) => None,
                    ParChild::Text(_) | ParChild::Quote { .. } => Some(true),
                    ParChild::Node(_) => Some(false),
                })
                .unwrap_or_default()
            && parent
                .flow
                .0
                .items()
                .rev()
                .find_map(|child| match child {
                    FlowChild::Spacing(_) => None,
                    FlowChild::Node(node) => Some(node.is::<ParNode>()),
                    FlowChild::Colbreak => Some(false),
                })
                .unwrap_or_default()
        {
            children.push_front(ParChild::Spacing(indent.into()));
        }

        parent.flow.par(ParNode(children), shared, !indent.is_zero());
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Accepts list / enum items, spaces, paragraph breaks.
struct ListBuilder<'a> {
    /// The list items collected so far.
    items: StyleVecBuilder<'a, ListItem>,
    /// Whether the list contains no paragraph breaks.
    tight: bool,
    /// Whether the list can be attached.
    attachable: bool,
    /// Trailing content for which it is unclear whether it is part of the list.
    staged: Vec<(&'a Content, StyleChain<'a>)>,
}

impl<'a> ListBuilder<'a> {
    fn accept(&mut self, content: &'a Content, styles: StyleChain<'a>) -> bool {
        if self.items.is_empty() {
            if content.is::<ParbreakNode>() {
                self.attachable = false;
            } else if !content.is::<SpaceNode>() && !content.is::<ListItem>() {
                self.attachable = true;
            }
        }

        if let Some(item) = content.downcast::<ListItem>() {
            if self
                .items
                .items()
                .next()
                .map_or(true, |first| item.kind() == first.kind())
            {
                self.items.push(item.clone(), styles);
                self.tight &= self.staged.drain(..).all(|(t, _)| !t.is::<ParbreakNode>());
            } else {
                return false;
            }
        } else if !self.items.is_empty()
            && (content.is::<SpaceNode>() || content.is::<ParbreakNode>())
        {
            self.staged.push((content, styles));
        } else {
            return false;
        }

        true
    }

    fn finish(self, parent: &mut Builder<'a>) -> SourceResult<()> {
        let (items, shared) = self.items.finish();
        let kind = match items.items().next() {
            Some(item) => item.kind(),
            None => return Ok(()),
        };

        let tight = self.tight;
        let attached = tight && self.attachable;
        let content = match kind {
            LIST => ListNode::<LIST> { tight, attached, items }.pack(),
            ENUM => ListNode::<ENUM> { tight, attached, items }.pack(),
            DESC | _ => ListNode::<DESC> { tight, attached, items }.pack(),
        };

        let stored = parent.scratch.templates.alloc(content);
        parent.accept(stored, shared)?;

        for (content, styles) in self.staged {
            parent.accept(content, styles)?;
        }

        parent.list.attachable = true;

        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Default for ListBuilder<'_> {
    fn default() -> Self {
        Self {
            items: StyleVecBuilder::default(),
            tight: true,
            attachable: true,
            staged: vec![],
        }
    }
}
