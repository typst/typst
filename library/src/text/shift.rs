use typst::model::SequenceNode;
use typst::util::EcoString;

use super::{variant, SpaceNode, TextNode, TextSize};
use crate::prelude::*;

/// Sub or superscript text.
///
/// The text is rendered smaller and its baseline is raised. To provide the best
/// typography possible, we first try to transform the text to superscript
/// codepoints. If that fails, we fall back to rendering shrunk normal letters
/// in a raised way.
#[derive(Debug, Hash)]
pub struct ShiftNode<const S: ShiftKind>(pub Content);

/// Shift the text into superscript.
pub type SuperNode = ShiftNode<SUPERSCRIPT>;

/// Shift the text into subscript.
pub type SubNode = ShiftNode<SUBSCRIPT>;

#[node(Show)]
impl<const S: ShiftKind> ShiftNode<S> {
    /// Whether to prefer the dedicated sub- and superscript characters of the
    /// font.
    pub const TYPOGRAPHIC: bool = true;
    /// The baseline shift for synthetic sub- and superscripts.
    pub const BASELINE: Length =
        Em::new(if S == SUPERSCRIPT { -0.5 } else { 0.2 }).into();
    /// The font size for synthetic sub- and superscripts.
    pub const SIZE: TextSize = TextSize(Em::new(0.6).into());

    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl<const S: ShiftKind> Show for ShiftNode<S> {
    fn unguard_parts(&self, _: Selector) -> Content {
        Self(self.0.clone()).pack()
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }

    fn realize(
        &self,
        world: Tracked<dyn World>,
        styles: StyleChain,
    ) -> SourceResult<Content> {
        let mut transformed = None;
        if styles.get(Self::TYPOGRAPHIC) {
            if let Some(text) = search_text(&self.0, S) {
                if is_shapable(world, &text, styles) {
                    transformed = Some(TextNode(text).pack());
                }
            }
        };

        Ok(transformed.unwrap_or_else(|| {
            let mut map = StyleMap::new();
            map.set(TextNode::BASELINE, styles.get(Self::BASELINE));
            map.set(TextNode::SIZE, styles.get(Self::SIZE));
            self.0.clone().styled_with_map(map)
        }))
    }
}

/// Find and transform the text contained in `content` to the given script kind
/// if and only if it only consists of `Text`, `Space`, and `Empty` leaf nodes.
fn search_text(content: &Content, mode: ShiftKind) -> Option<EcoString> {
    if content.is_empty() {
        Some(EcoString::new())
    } else if content.is::<SpaceNode>() {
        Some(' '.into())
    } else if let Some(text) = content.downcast::<TextNode>() {
        convert_script(&text.0, mode)
    } else if let Some(seq) = content.downcast::<SequenceNode>() {
        let mut full = EcoString::new();
        for item in seq.0.iter() {
            match search_text(item, mode) {
                Some(text) => full.push_str(&text),
                None => return None,
            }
        }
        Some(full)
    } else {
        None
    }
}

/// Checks whether the first retrievable family contains all code points of the
/// given string.
fn is_shapable(world: Tracked<dyn World>, text: &str, styles: StyleChain) -> bool {
    for family in styles.get(TextNode::FAMILY).0.iter() {
        if let Some(font) = world
            .book()
            .select(family.as_str(), variant(styles))
            .and_then(|id| world.font(id))
        {
            return text.chars().all(|c| font.ttf().glyph_index(c).is_some());
        }
    }

    false
}

/// Convert a string to sub- or superscript codepoints if all characters
/// can be mapped to such a codepoint.
fn convert_script(text: &str, mode: ShiftKind) -> Option<EcoString> {
    let mut result = EcoString::with_capacity(text.len());
    let converter = match mode {
        SUPERSCRIPT => to_superscript_codepoint,
        SUBSCRIPT | _ => to_subscript_codepoint,
    };

    for c in text.chars() {
        match converter(c) {
            Some(c) => result.push(c),
            None => return None,
        }
    }

    Some(result)
}

/// Convert a character to its corresponding Unicode superscript.
fn to_superscript_codepoint(c: char) -> Option<char> {
    char::from_u32(match c {
        '0' => 0x2070,
        '1' => 0x00B9,
        '2' => 0x00B2,
        '3' => 0x00B3,
        '4'..='9' => 0x2070 + (c as u32 + 4 - '4' as u32),
        '+' => 0x207A,
        '-' => 0x207B,
        '=' => 0x207C,
        '(' => 0x207D,
        ')' => 0x207E,
        'n' => 0x207F,
        'i' => 0x2071,
        ' ' => 0x0020,
        _ => return None,
    })
}

/// Convert a character to its corresponding Unicode subscript.
fn to_subscript_codepoint(c: char) -> Option<char> {
    char::from_u32(match c {
        '0' => 0x2080,
        '1'..='9' => 0x2080 + (c as u32 - '0' as u32),
        '+' => 0x208A,
        '-' => 0x208B,
        '=' => 0x208C,
        '(' => 0x208D,
        ')' => 0x208E,
        'a' => 0x2090,
        'e' => 0x2091,
        'o' => 0x2092,
        'x' => 0x2093,
        'h' => 0x2095,
        'k' => 0x2096,
        'l' => 0x2097,
        'm' => 0x2098,
        'n' => 0x2099,
        'p' => 0x209A,
        's' => 0x209B,
        't' => 0x209C,
        ' ' => 0x0020,
        _ => return None,
    })
}

/// A category of script.
pub type ShiftKind = usize;

/// Text that is rendered smaller and raised, also known as superior.
const SUPERSCRIPT: ShiftKind = 0;

/// Text that is rendered smaller and lowered, also known as inferior.
const SUBSCRIPT: ShiftKind = 1;
