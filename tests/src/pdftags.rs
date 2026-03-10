use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{Arc, LazyLock};

use ecow::eco_format;
use hayro_syntax::content::ops::TypedInstruction;
use hayro_syntax::object::dict::keys;
use hayro_syntax::object::{Array, Dict, Name, Object, Stream};
use hayro_syntax::object::{Number, ObjRef};
use indexmap::IndexMap;
use roxmltree::{Document, Node};
use typst::diag::{StrResult, bail};

/// The context used while formatting the PDF tag tree.
struct Formatter<'a> {
    pages: &'a IndexMap<ObjRef, PageContent<'a>>,
    buf: String,
    indent: usize,
    /// Whether the last character written was a newline and an indent should
    /// be applied to the next line. This isn't done directly because the
    /// indent might be changed before writing any other content.
    pending_indent: bool,
    /// A string that is used to pad the following content, if there is any
    /// on the same line. If a newline is written, discard this padding so there
    /// isn't any trailing whitespace.
    maybe_space: Option<&'static str>,
}

/// The marked content sequences in a PDF page.
struct PageContent<'a> {
    idx: usize,
    marked_content: HashMap<i64, MarkedContent<'a>>,
}

/// The properties of a marked content sequence.
struct MarkedContent<'a> {
    props: Dict<'a>,
}

impl<'a> Formatter<'a> {
    /// Create a new formatter.
    pub fn new(pages: &'a IndexMap<ObjRef, PageContent<'a>>) -> Self {
        Self {
            pages,
            buf: String::new(),
            indent: 0,
            pending_indent: false,
            maybe_space: None,
        }
    }
}

impl std::fmt::Write for Formatter<'_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.pending_indent {
            let indent = 2 * self.indent;
            write!(&mut self.buf, "{:indent$}", "").ok();
            self.pending_indent = false;
        }

        if let Some(space) = self.maybe_space.take()
            && !s.starts_with('\n')
        {
            self.buf.push_str(space);
        }

        let mut lines = s.lines();
        if let Some(first) = lines.next() {
            self.buf.push_str(first);
        }

        for l in lines {
            let indent = 2 * self.indent;
            write!(&mut self.buf, "\n{:indent$}", "").ok();
            self.buf.push_str(l);
        }

        if s.ends_with('\n') {
            self.buf.push('\n');
            self.pending_indent = true;
        }

        Ok(())
    }
}

/// Format the tag tree of a PDF document as YAML.
pub fn format(doc: &[u8]) -> StrResult<String> {
    let pdf = hayro_syntax::Pdf::new(Arc::new(doc.to_vec()))
        .map_err(|e| eco_format!("couldn't load PDF: {e:?}"))?;
    let catalog_ref = pdf.xref().root_id();
    let catalog = pdf.xref().get::<Dict>(catalog_ref).ok_or("missing catalog")?;

    let pages = catalog.get::<Dict>(keys::PAGES).ok_or("missing pages")?;
    let page_array = pages.get::<Array>(keys::KIDS).ok_or("missing page kids")?;
    let page_refs = page_array
        .raw_iter()
        .map(|o| o.as_obj_ref().ok_or("expected page obj ref"));
    let page_contents = pdf
        .pages()
        .iter()
        .enumerate()
        .zip(page_refs)
        .map(|((idx, page), page_ref)| {
            let page_ref = page_ref?;
            let marked_content = page
                .typed_operations()
                .filter_map(|op| match op {
                    TypedInstruction::MarkedContentPointWithProperties(mc) => {
                        let props = mc.1.into_dict()?;
                        let mcid = props.get(keys::MCID)?;
                        Some((mcid, MarkedContent { props }))
                    }
                    TypedInstruction::BeginMarkedContentWithProperties(mc) => {
                        let props = mc.1.into_dict()?;
                        let mcid = props.get(keys::MCID)?;
                        Some((mcid, MarkedContent { props }))
                    }
                    _ => None,
                })
                .collect();

            Ok((page_ref, PageContent { idx, marked_content }))
        })
        .collect::<StrResult<_>>()?;

    let struct_tree: Dict =
        catalog.get(keys::STRUCT_TREE_ROOT).ok_or("struct tree root")?;
    let kids = struct_tree.get::<Array>(keys::K).ok_or("struct tree kids")?;
    let document = kids.iter::<Dict>().next().ok_or("document tag")?;
    let kids = document.get::<Array>(keys::K).ok_or("document kids")?;

    let mut f = Formatter::new(&page_contents);
    if let Some(stream) = catalog.get::<Stream>(keys::METADATA) {
        format_xmp_metadata(&mut f, stream)
            .map_err(|e| eco_format!("error formatting XMP metadata: {e}"))?;
    }

    format_tag_children(&mut f, &document, &kids)?;
    Ok(f.buf)
}

/// Format the XMP metadata of a PDF document.
fn format_xmp_metadata(f: &mut Formatter, stream: Stream) -> StrResult<()> {
    fn child<'a, 'input>(
        node: &Node<'a, 'input>,
        name: &str,
    ) -> Option<Node<'a, 'input>> {
        node.children().find(|c| c.tag_name().name() == name)
    }

    let bytes = stream
        .decoded()
        .map_err(|e| eco_format!("failed to decode stream: {e:?}"))?;
    let xml_str =
        std::str::from_utf8(&bytes).map_err(|e| eco_format!("invalid UTF-8: {e}"))?;
    let xmp = Document::parse(xml_str).map_err(|e| eco_format!("invalid XML: {e}"))?;

    let xmpmeta = xmp.root_element();
    let rdf = child(&xmpmeta, "RDF").ok_or("missing rdf:RDF")?;
    let description = child(&rdf, "Description").ok_or("missing rdf:Description")?;

    if let Some(dc_lang) = child(&description, "language") {
        let bag = child(&dc_lang, "Bag").ok_or("missing rdf:Bag")?;
        let item = bag.first_element_child().ok_or("missing rdf:li")?;
        assert_eq!(item.tag_name().name(), "li");
        let lang = item.text().ok_or("missing text")?;
        // Only write language if it deviates from the default.
        if lang != "en" {
            writeln!(f, "lang: {lang:?}").ok();
        }
    }

    if !f.buf.is_empty() {
        writeln!(f, "---").ok();
    }

    Ok(())
}

/// Format a PDF structure element (tag).
fn format_tag(f: &mut Formatter, tag: &Dict) -> StrResult<()> {
    assert_type(tag, keys::STRUCT_ELEM)?;

    let ty = tag.get::<Name>(keys::S).ok_or("missing structure type")?;
    let ty = ty.as_str();
    writeln!(f, "- Tag: {ty}").ok();

    f.indent += 1;

    format_tag_attrs(f, tag).map_err(|e| eco_format!("{ty}: {e}"))?;

    let Some(kids) = tag.get::<Array>(keys::K) else { bail!("{ty}: missing kids array") };
    if kids.raw_iter().next().is_some() {
        writeln!(f, "/K:").ok();

        f.indent += 1;
        format_tag_children(f, tag, &kids).map_err(|e| eco_format!("{ty}: {e}"))?;
        f.indent -= 1;
    }

    f.indent -= 1;

    Ok(())
}

/// Format either a child structure element (tag), a marked-content sequence or
/// an annotation.
fn format_tag_children(f: &mut Formatter, tag: &Dict, kids: &Array) -> StrResult<()> {
    for kid in kids.iter::<Object>() {
        match kid {
            Object::Number(mcid) => {
                let page_ref = tag
                    .get_ref(keys::PG)
                    .ok_or("missing page ref on structure element")?;
                format_marked_content(f, page_ref, mcid.as_i64())
                    .map_err(|e| eco_format!("error formatting marked-content: {e}"))?;
            }
            Object::Dict(dict) => {
                let ty = dict.get::<Name>(keys::TYPE).ok_or("missing object type")?;
                match &*ty {
                    b"MCR" => {
                        let mcid = dict.get(keys::MCID).ok_or("missing content id")?;
                        let page_ref = dict
                            .get_ref(keys::PG)
                            .ok_or("missing page ref on marked-content reference")?;
                        format_marked_content(f, page_ref, mcid).map_err(|e| {
                            eco_format!("error formatting marked-content: {e}")
                        })?;
                    }
                    keys::OBJR => {
                        let annot = dict
                            .get::<Dict>(keys::OBJ)
                            .ok_or("missing referenced obj")?;
                        let page_ref = dict
                            .get_ref(keys::PG)
                            .ok_or("missing page ref on object reference")?;
                        format_annotation(f, page_ref, &annot).map_err(|e| {
                            eco_format!("error formatting annotation: {e}")
                        })?;
                    }
                    _ => format_tag(f, &dict)?,
                }
            }
            _ => bail!("unexpected object {kid:?}"),
        }
    }

    Ok(())
}

/// Format a marked content sequence.
fn format_marked_content(
    f: &mut Formatter,
    page_ref: ObjRef,
    mcid: i64,
) -> StrResult<()> {
    let page = &f.pages[&page_ref];
    let page_idx = page.idx;
    let mc = &page.marked_content[&mcid];
    writeln!(f, "- Content: page={page_idx} mcid={mcid}").ok();

    f.indent += 1;
    if let Some(val) = mc.props.get::<Object>(keys::LANG) {
        format_attr(f, "Lang", val, format_str)?;
    }
    if let Some(val) = mc.props.get::<Object>(keys::ALT) {
        format_attr(f, "Alt", val, format_str)?;
    }
    if let Some(val) = mc.props.get::<Object>(keys::E) {
        format_attr(f, "Expanded", val, format_str)?;
    }
    if let Some(val) = mc.props.get::<Object>(keys::ACTUAL_TEXT) {
        format_attr(f, "ActualText", val, format_str)?;
    }
    f.indent -= 1;

    Ok(())
}

/// Format a link annotation.
fn format_annotation(f: &mut Formatter, page_ref: ObjRef, annot: &Dict) -> StrResult<()> {
    assert_type(annot, keys::ANNOT)?;

    let subtype = annot.get::<Name>(keys::SUBTYPE).ok_or("missing subtype")?;
    let subtype = subtype.as_str();

    let page = &f.pages[&page_ref];
    let page_idx = page.idx;
    writeln!(f, "- Annotation: page={page_idx} subtype={subtype}").ok();

    f.indent += 1;
    if let Some(val) = annot.get::<Object>(keys::CONTENTS) {
        format_attr(f, "Contents", val, format_str)?;
    }
    if let Some(action) = annot.get::<Dict>(keys::A) {
        assert_type(&action, b"Action")?;
        if let Some(val) = action.get(keys::URI) {
            format_attr(f, "URI", val, format_str)?;
        }
    }
    f.indent -= 1;

    Ok(())
}

/// Format the attributes of a structure element (tag).
fn format_tag_attrs(f: &mut Formatter, tag: &Dict) -> StrResult<()> {
    if let Some(val) = tag.get::<Object>(keys::ID) {
        format_attr(f, "Id", val, format_byte_str)?;
    }
    if let Some(val) = tag.get::<Object>(keys::LANG) {
        format_attr(f, "Lang", val, format_str)?;
    }
    if let Some(val) = tag.get::<Object>(keys::ALT) {
        format_attr(f, "Alt", val, format_str)?;
    }
    if let Some(val) = tag.get::<Object>(keys::E) {
        format_attr(f, "Expanded", val, format_str)?;
    }
    if let Some(val) = tag.get::<Object>(keys::ACTUAL_TEXT) {
        format_attr(f, "ActualText", val, format_str)?;
    }
    if let Some(val) = tag.get::<Object>(keys::T) {
        format_attr(f, "T", val, format_str)?;
    }

    if let Some(attrs_array) = tag.get::<Array>(keys::A) {
        for attrs in attrs_array.iter::<Dict>() {
            // Attributes are stored in a hash map. Sort the keys by a
            // predefined order so the formatted tag tree is deterministic.
            let attribute_map = &*ATTRIBUTE_MAP;
            let mut indices = attrs
                .keys()
                .filter(|name| {
                    // Ignore the attribute owner
                    name.as_str() != "O"
                })
                .map(|name| {
                    let Some(idx) = attribute_map.get(name.as_str()) else {
                        bail!("unhandled key `{}`", name.as_str())
                    };
                    Ok(*idx)
                })
                .collect::<StrResult<Vec<_>>>()?;
            indices.sort();

            for idx in indices {
                let (name, fmt) = ATTRIBUTES[idx];
                let val = attrs.get::<Object>(name.as_bytes()).unwrap();
                format_attr(f, name, val, fmt)?;
            }
        }
    }

    Ok(())
}

type FmtValFn = fn(f: &mut Formatter, val: &Object) -> Result<(), ()>;

/// The sorted list of PDF structure element attributes and how to format them.
const ATTRIBUTES: [(&str, FmtValFn); 36] = [
    // List
    ("ListNumbering", format_name),
    // Table
    ("Summary", format_str),
    ("Scope", format_name),
    ("Headers", |f, val| format_array(f, val, format_byte_str)),
    ("RowSpan", format_int),
    ("ColSpan", format_int),
    // Layout
    ("Placement", format_name),
    ("WritingMode", format_name),
    ("BBox", |f, val| format_array(f, val, format_float)),
    ("Width", format_float),
    ("Height", format_float),
    ("BackgroundColor", format_color),
    ("BorderColor", |f, val| format_sides(f, val, format_color)),
    ("BorderStyle", |f, val| format_sides(f, val, format_name)),
    ("BorderThickness", |f, val| format_sides(f, val, format_float)),
    ("Padding", |f, val| format_sides(f, val, format_float)),
    ("Color", format_color),
    ("SpaceBefore", format_float),
    ("SpaceAfter", format_float),
    ("StartIndent", format_float),
    ("EndIndent", format_float),
    ("TextIndent", format_float),
    ("TextAlign", format_name),
    ("BlockAlign", format_name),
    ("InlineAlign", format_name),
    ("TBorderStyle", |f, val| format_sides(f, val, format_name)),
    ("TPadding", |f, val| format_sides(f, val, format_float)),
    ("BaselineShift", format_float),
    ("LineHeight", |f, val| format_one_of(f, val, [format_float, format_name])),
    ("TextDecorationColor", format_color),
    ("TextDecorationThickness", format_float),
    ("TextDecorationType", format_name),
    ("GlyphOrientationVertical", |f, val| {
        format_one_of(f, val, [format_int, format_name])
    }),
    ("ColumnCount", format_int),
    ("ColumnGap", |f, val| format_array(f, val, format_float)),
    ("ColumnWidths", |f, val| format_array(f, val, format_float)),
];

/// A lookup table from an attribute name to its index in the attribute array.
static ATTRIBUTE_MAP: LazyLock<HashMap<&str, usize>> =
    LazyLock::new(|| ATTRIBUTES.into_iter().map(|(name, _)| name).zip(0..).collect());

fn format_attr(
    f: &mut Formatter,
    name: &str,
    val: Object,
    fmt: FmtValFn,
) -> StrResult<()> {
    write!(f, "/{name}:").ok();
    f.maybe_space = Some(" ");
    f.indent += 1;
    if fmt(f, &val).is_err() {
        bail!("couldn't format `{name}`: `{val:?}`");
    }
    f.indent -= 1;
    writeln!(f).ok();
    Ok(())
}

fn format_int(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    let Object::Number(val) = val else { return Err(()) };
    write!(f, "{}", val.as_i64()).ok();
    Ok(())
}

fn format_float(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    let Object::Number(val) = val else { return Err(()) };
    write!(f, "{:7.3}", val.as_f64()).ok();
    Ok(())
}

fn format_name(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    let Object::Name(val) = val else { return Err(()) };
    f.write_str(val.as_str()).ok();
    Ok(())
}

fn format_str(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    const UTF_16_BE_BOM: [u8; 2] = *b"\xFE\xFF";

    let Object::String(val) = val else { return Err(()) };
    let bytes = val.get();

    if let Some(data) = bytes.strip_prefix(&UTF_16_BE_BOM) {
        let code_units = data.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]]));
        let str = std::char::decode_utf16(code_units)
            .collect::<Result<String, _>>()
            .unwrap();
        write!(f, "{str:?}").ok();
    } else {
        let str = std::str::from_utf8(&bytes).unwrap();
        write!(f, "{str:?}").ok();
    }
    Ok(())
}

fn format_byte_str(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    let Object::String(val) = val else { return Err(()) };
    let bytes = val.get();
    if bytes.iter().all(
        |b| matches!(b, b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b' ' | b'-' | b'_'),
    ) {
        let str = std::str::from_utf8(&bytes).unwrap();
        write!(f, "{str:?}").ok();
    } else {
        write!(f, "0x").ok();
        for b in bytes.iter() {
            write!(f, "{b:02x}").ok();
        }
    }
    Ok(())
}

fn format_color(f: &mut Formatter, val: &Object) -> Result<(), ()> {
    let Object::Array(array) = val else { return Err(()) };
    if array.raw_iter().count() != 3 {
        return Err(());
    };
    let mut iter = array.iter::<Number>();
    let [r, g, b] = std::array::from_fn(|_| {
        let n = iter.next().unwrap().as_f64();
        (255.0 * n).round() as u8
    });
    write!(f, "#{r:02x}{g:02x}{b:02x}").ok();
    Ok(())
}

fn format_array(f: &mut Formatter, val: &Object, fmt: FmtValFn) -> Result<(), ()> {
    match val {
        Object::Array(array) => {
            f.indent += 1;
            write!(f, "[").ok();
            for (i, val) in array.iter().enumerate() {
                if i != 0 {
                    write!(f, ", ").ok();
                }
                fmt(f, &val)?;
            }
            write!(f, "]").ok();
            f.indent -= 1;
            Ok(())
        }
        val => fmt(f, val),
    }
}

fn format_sides(f: &mut Formatter, val: &Object, fmt: FmtValFn) -> Result<(), ()> {
    match val {
        Object::Array(array) if array.raw_iter().count() == 4 => {
            let mut iter = array.iter::<Object>();
            let values: [_; 4] = std::array::from_fn(|_| iter.next().unwrap());

            #[rustfmt::skip]
            const NAMES: [(&str, &str); 4] = [
                ("before", " "),
                ("after", "  "),
                ("start", "  "),
                ("end", "    "),
            ];
            for ((name, space), val) in NAMES.iter().zip(values) {
                write!(f, "\n{name}:").ok();
                f.maybe_space = Some(space);
                f.indent += 1;
                fmt(f, &val)?;
                f.indent -= 1;
            }
            Ok(())
        }
        val => fmt(f, val),
    }
}

fn format_one_of<const N: usize>(
    f: &mut Formatter,
    val: &Object,
    fmt: [FmtValFn; N],
) -> Result<(), ()> {
    for fmt in fmt {
        if fmt(f, val).is_ok() {
            return Ok(());
        }
    }
    Err(())
}

#[track_caller]
fn assert_type(obj: &Dict, expected: &[u8]) -> StrResult<()> {
    let ty = obj.get::<Name>(keys::TYPE).ok_or("missing object type")?;
    if &*ty != expected {
        let found = ty.as_str();
        let expected = std::str::from_utf8(expected).unwrap();
        bail!("expected object type `{expected}` found `{found}`")
    }
    Ok(())
}
