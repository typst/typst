use std::collections::HashMap;

use typst::{
    geom::GenAlign,
    model::{Content, StyleChain}, doc::Destination,
};

use typst_library::{
    layout::{
        AlignElem, BlockElem, EnumItem, HElem, ListItem, ParbreakElem,
        TableElem, TermItem, VElem,
    },
    math::EquationElem,
    meta::{HeadingElem, LinkElem, LinkTarget},
    text::{EmphElem, LinebreakElem, RawElem, SpaceElem, StrongElem, TextElem, SmartQuoteElem},
    visualize::ImageElem,
};
use pandoc_types::definition::{self as p};

#[derive(Debug)]
// pandoc separates the concepts of Inlines and Blocks but typst doesn't
enum BlockOrInlines {
    Blocks(Vec<p::Block>),
    Inlines(Vec<p::Inline>),
}
impl From<p::Inline> for BlockOrInlines {
    fn from(value: p::Inline) -> Self {
        BlockOrInlines::Inlines(vec![value])
    }
}
impl From<p::Block> for BlockOrInlines {
    fn from(value: p::Block) -> Self {
        BlockOrInlines::Blocks(vec![value])
    }
}

pub fn pandoc(content: &Content) -> Vec<u8> {
    // eprintln!("input: {:?}", content);
    serde_json::to_vec(&serde_json::json!(pandoc_types::definition::Pandoc {
        meta: HashMap::new(),
        blocks: to_block(to_pandoc_ast(content))
    }))
    .unwrap()
}

fn to_inline(v: &BlockOrInlines) -> Result<Vec<p::Inline>, String> {
    match v {
        BlockOrInlines::Blocks(b) => Err(format!("not inline: {:?}", b)),
        BlockOrInlines::Inlines(i) => Ok(i.clone()),
    }
}
fn to_block(v: BlockOrInlines) -> Vec<p::Block> {
    match v {
        BlockOrInlines::Blocks(b) => b,
        BlockOrInlines::Inlines(i) => vec![p::Block::Plain(i)],
    }
}

fn to_pandoc_ast(content: &Content) -> BlockOrInlines {
    if let Some((e, _)) = content.to_styled() {
        return to_pandoc_ast(e);
    }
    if let Some(t) = content.to::<TextElem>() {
        return p::Inline::Str(t.text().to_string()).into();
    }
    if let Some(_) = content.to::<SpaceElem>() {
        return p::Inline::Space.into();
    }
    if let Some(_) = content.to::<HElem>() {
        // pandoc doesn't really handle flexible horizontal spacing
        return p::Inline::Space.into();
    }
    if let Some(_) = content.to::<VElem>() {
        // pandoc doesn't really handle flexible vertical spacing, return empty para
        return p::Block::Para(vec![]).into();
    }
    if let Some(e) = content.to::<EmphElem>() {
        return p::Inline::Emph(
            to_inline(&to_pandoc_ast(&e.body())).expect("content of emph not inline"),
        )
        .into();
    }
    if let Some(e) = content.to::<EquationElem>() {
        // todo: pandoc represents math as latex math. parse typst equations into latex math?
        return p::Inline::Math(
            if e.block(StyleChain::default()) {
                p::MathType::DisplayMath
            } else {
                p::MathType::InlineMath
            },
            r#"\text{\[equation\]}"#.to_string(),
        )
        .into();
    }
    if let Some(e) = content.to::<ImageElem>() {
        return p::Inline::Image(
            p::Attr::default(),
            vec![],
            p::Target { url: e.path().to_string(), title: "".to_string() },
        )
        .into();
    }
    if let Some(_) = content.to::<LinebreakElem>() {
        return (p::Inline::LineBreak).into();
    }
    if let Some(e) = content.to::<StrongElem>() {
        return p::Inline::Strong(
            to_inline(&to_pandoc_ast(&e.body())).expect("content of strong not inline"),
        )
        .into();
    }
    if let Some(e) = content.to::<LinkElem>() {
        let dest = e.dest();
        let dstr = match dest {
            LinkTarget::Dest(Destination::Url(u)) => u.to_string(),
            LinkTarget::Dest(Destination::Position(p)) => format!("#pos:{:?}", p),
            LinkTarget::Dest(Destination::Location(p)) => format!("#loc:{:?}", p),
            LinkTarget::Label(l) => format!("#label:{:?}", l),
        };
        return p::Inline::Link(
            p::Attr::default(),
            to_inline(&to_pandoc_ast(&e.body())).expect("content of href not inline"),
            p::Target { title: "".to_string(), url: dstr },
        )
        .into();
    }
    if let Some(e) = content.to::<RawElem>() {
        let mut a = p::Attr::default();
        if let Some(l) = e.lang(StyleChain::default()) {
            a.classes.push(l.to_string());
        }
        if e.block(StyleChain::default()) {
            return p::Block::CodeBlock(a, e.text().to_string()).into();
        } else {
            return p::Inline::Code(a, e.text().to_string()).into();
        }
    }
    if let Some(e) = content.to::<BlockElem>() {
        // todo: parse width, height, styling?
        return p::Block::Div(
            p::Attr::default(),
            to_block(
                e.body(StyleChain::default())
                    .map(|e| to_pandoc_ast(&e))
                    .unwrap_or(BlockOrInlines::Blocks(vec![])),
            ),
        )
        .into();
    }
    if let Some(e) = content.to::<AlignElem>() {
        let mut attr = p::Attr::default();
        let align = e.alignment(StyleChain::default());
        let horiz_align = align.x;
        let vert_align = align.y;
        if horiz_align != GenAlign::Start {
            attr.attributes
                .push(("align".to_string(), format!("{:?}", horiz_align)));
        }
        if vert_align != GenAlign::Specific(typst::geom::Align::Top) {
            // vertical align is kinda meaningless in the pandoc document model
            attr.attributes
                .push(("vertical-align".to_string(), format!("{:?}", vert_align)));
        }
        return p::Block::Div(attr, to_block(to_pandoc_ast(&e.body()))).into();
    }

    // typst does not track which list an item belongs to in the AST, it only does this in the layout phase with ListBuilder.
    // but we skip the layout phase. todo: invoke ListBuilder ourselves during handling of sequences (?)
    if let Some(e) = content.to::<ListItem>() {
        return p::Block::BulletList(vec![to_block(to_pandoc_ast(&e.body()))]).into();
    }
    if let Some(e) = content.to::<EnumItem>() {
        return p::Block::OrderedList(
            p::ListAttributes::default(),
            vec![to_block(to_pandoc_ast(&e.body()))],
        )
        .into();
    }
    if let Some(e) = content.to::<TermItem>() {
        return p::Block::DefinitionList(vec![(
            to_inline(&to_pandoc_ast(&e.term())).expect("term item not inlines"),
            vec![to_block(to_pandoc_ast(&e.description()))],
        )])
        .into();
    }
    if let Some(e) = content.to::<SmartQuoteElem>() {
        // todo: use state machine?
        let t = if e.double(StyleChain::default()) {'"'} else {'\''};
        return p::Inline::Str(t.to_string()).into();
    }
    if let Some(e) = content.to::<TableElem>() {
        let body: Vec<_> = e
            .children()
            .chunks_exact(e.columns(StyleChain::default()).0.len())
            .map(|row| p::Row {
                attr: p::Attr::default(),
                cells: row
                    .into_iter()
                    .map(to_pandoc_ast)
                    .map(|content| p::Cell {
                        content: to_block(content),
                        ..Default::default()
                    })
                    .collect(),
            })
            .collect();
        //let header = body.remove(0); // typst doesn't have a concept of header rows but pandoc needs them
        let body = p::TableBody { body, ..Default::default() };
        let tbl = p::Table {
            //head: p::TableHead { rows: vec![header], attr: p::Attr::default() },
            bodies: vec![body],
            colspecs: e
                .columns(StyleChain::default())
                .0
                .into_iter()
                .map(|_| {
                    // todo: convert sizing and alignment
                    p::ColSpec::default()
                })
                .collect(),
            ..Default::default()
        };
        return p::Block::Table(tbl).into();
    }

    if let Some(l) = content.to_sequence() {
        let mut blocks = vec![];
        let mut latest_inlines: Vec<p::Inline> = vec![];
        for ele in l {
            if ele.is::<ParbreakElem>() {
                let inlines = std::mem::replace(&mut latest_inlines, vec![]);
                blocks.push(p::Block::Para(inlines));
            } else {
                match to_pandoc_ast(ele) {
                    BlockOrInlines::Blocks(b) => {
                        if latest_inlines.len() > 0 {
                            let inlines = std::mem::replace(&mut latest_inlines, vec![]);

                            blocks.push(p::Block::Plain(inlines));
                        }
                        blocks.extend(b);
                    }
                    BlockOrInlines::Inlines(i) => latest_inlines.extend(i),
                }
            }
        }
        if latest_inlines.len() > 0 {
            if blocks.len() == 0 {
                // no blocks at all, everything inline
                return BlockOrInlines::Inlines(latest_inlines);
            }
            let inlines = std::mem::replace(&mut latest_inlines, vec![]);

            blocks.push(p::Block::Plain(inlines));
        }
        return BlockOrInlines::Blocks(blocks);
    }
    if let Some(h) = content.to::<HeadingElem>() {
        let level = h.level(StyleChain::default()).get().try_into().unwrap(); // why is stylechain is needed here?
        return p::Block::Header(
            level,
            p::Attr::default(),
            to_inline(&to_pandoc_ast(&h.body())).expect("heading content not inline"),
        )
        .into();
    }
    eprintln!("conversion not implemented for {:?}", content);
    return p::Inline::Str("[unk]".to_string()).into();
}
