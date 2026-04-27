use ecow::eco_format;

use crate::{HtmlElement, HtmlNode, attr};

/// Turns CSS properties on all elements in the DOM into inline `style`
/// attributes.
///
/// This by itself would not need a separate pass over the DOM, but it will be
/// supplanted by more advanced CSS handling, so it makes sense to already
/// organize the code like this.
pub fn resolve_inline_styles(root: &mut HtmlElement) {
    visit_elem(root);
}

fn visit_elem(elem: &mut HtmlElement) {
    if !elem.css.is_empty() {
        // TODO: Use to_eco_string once merged:
        // https://github.com/typst/ecow/pull/60
        let mut generated = eco_format!("{}", elem.css.to_inline());
        if let Some(style) = elem.attrs.get_mut(attr::style) {
            if !style.is_empty() {
                generated.push_str("; ");
            }
            // TODO: Use insert_str once merged:
            // https://github.com/typst/ecow/pull/59
            generated.push_str(style);
            *style = generated;
        } else {
            elem.attrs.push(attr::style, generated);
        }
    }

    for child in elem.children.make_mut().iter_mut() {
        if let HtmlNode::Element(elem) = child {
            visit_elem(elem);
        }
    }
}
