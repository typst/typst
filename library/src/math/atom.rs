use super::*;

/// # Atom
/// An atom in a math formula: `x`, `+`, `12`.
///
/// ## Parameters
/// - text: EcoString (positional, required)
///   The atom's text.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AtomNode(pub EcoString);

#[node]
impl AtomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("text")?).pack())
    }
}

impl Texify for AtomNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        let multi = self.0.graphemes(true).count() > 1;
        if multi {
            t.push_str("\\mathrm{");
        }

        for c in self.0.chars() {
            let supportive = c == '|';
            if supportive {
                t.support();
            }
            t.push_escaped(c);
            if supportive {
                t.support();
            }
        }

        if multi {
            t.push_str("}");
        }

        Ok(())
    }
}
