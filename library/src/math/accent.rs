use super::*;

/// # Accent
/// An accented node.
///
/// ## Example
/// ```
/// $acc(a, ->) != acc(a, ~)$ \
/// $acc(a, `) = acc(a, grave)$
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to which the accent is applied.
///   May consist of multiple letters.
///
///   ### Example
///   ```
///   $acc(A B C, ->)$
///   ```
///
/// - accent: Content (positional, required)
///   The accent to apply to the base.
///
///   Supported accents include:
///   - Grave: `` ` ``
///   - Acute: `´`
///   - Circumflex: `^`
///   - Tilde: `~`
///   - Macron: `¯`
///   - Overline: `‾`
///   - Breve: `˘`
///   - Dot: `.`
///   - Diaeresis: `¨`
///   - Caron: `ˇ`
///   - Arrow: `→`
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AccNode {
    /// The accent base.
    pub base: Content,
    /// The Unicode accent character.
    pub accent: char,
}

#[node]
impl AccNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let Spanned { v, span } = args.expect::<Spanned<Content>>("accent")?;
        let accent = match extract(&v) {
            Some(Ok(c)) => c,
            Some(Err(msg)) => bail!(span, "{}", msg),
            None => bail!(span, "not an accent"),
        };
        Ok(Self { base, accent }.pack())
    }
}

#[rustfmt::skip]
fn extract(content: &Content) -> Option<Result<char, &'static str>> {
    let MathNode { children, .. } = content.to::<MathNode>()?;
    let [child] = children.as_slice() else { return None };
    let c = if let Some(atom) = child.to::<AtomNode>() {
        let mut chars = atom.0.chars();
        chars.next().filter(|_| chars.next().is_none())?
    } else if let Some(symbol) = child.to::<SymbolNode>() {
        match symmie::get(&symbol.0) {
            Some(c) => c,
            None => return Some(Err("unknown symbol")),
        }
    } else {
        return None;
    };

    Some(Ok(match c {
        '`' | '\u{300}' => '\u{300}',              // Grave
        '´' | '\u{301}' => '\u{301}',              // Acute
        '^' | '\u{302}' => '\u{302}',              // Circumflex
        '~' | '\u{223C}' | '\u{303}' => '\u{303}', // Tilde
        '¯' | '\u{304}' => '\u{304}',              // Macron
        '‾' | '\u{305}' => '\u{305}',              // Overline
        '˘' | '\u{306}' => '\u{306}',              // Breve
        '.' | '\u{22C5}' | '\u{307}' => '\u{307}', // Dot
        '¨' | '\u{308}' => '\u{308}',              // Diaeresis
        'ˇ' | '\u{30C}' => '\u{30C}',              // Caron
        '→' | '\u{20D7}' => '\u{20D7}',            // Arrow
        _ => return None,
    }))
}

impl Texify for AccNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        if let Some(sym) = unicode_math::SYMBOLS.iter().find(|sym| {
            sym.codepoint == self.accent
                && sym.atom_type == unicode_math::AtomType::Accent
        }) {
            t.push_str("\\");
            t.push_str(sym.name);
            t.push_str("{");
            self.base.texify(t)?;
            t.push_str("}");
        } else {
            self.base.texify(t)?;
        }
        Ok(())
    }
}
