use typst::diag::{StrResult, bail};
use typst::foundations::{Binding, Func};

use crate::{GROUPS, LIBRARY, get_module};

/// Resolve an intra-doc link.
pub fn resolve(link: &str, base: &str) -> StrResult<String> {
    if link.starts_with('#') || link.starts_with("http") {
        return Ok(link.to_string());
    }

    let (head, tail) = split_link(link)?;
    let mut route = match resolve_known(head, base) {
        Some(route) => route,
        None => resolve_definition(head, base)?,
    };

    if !tail.is_empty() {
        route.push('/');
        route.push_str(tail);
    }

    if !route.contains(['#', '?']) && !route.ends_with('/') {
        route.push('/');
    }

    Ok(route)
}

/// Split a link at the first slash.
fn split_link(link: &str) -> StrResult<(&str, &str)> {
    let first = link.split('/').next().unwrap_or(link);
    let rest = link[first.len()..].trim_start_matches('/');
    Ok((first, rest))
}

/// Resolve a `$` link head to a known destination.
fn resolve_known(head: &str, base: &str) -> Option<String> {
    Some(match head {
        "$tutorial" => format!("{base}tutorial"),
        "$reference" => format!("{base}reference"),
        "$category" => format!("{base}reference"),
        "$syntax" => format!("{base}reference/syntax"),
        "$styling" => format!("{base}reference/styling"),
        "$scripting" => format!("{base}reference/scripting"),
        "$context" => format!("{base}reference/context"),
        "$html" => format!("{base}reference/html"),
        "$pdf" => format!("{base}reference/pdf"),
        "$guides" => format!("{base}guides"),
        "$changelog" => format!("{base}changelog"),
        "$universe" => "https://typst.app/universe".into(),
        _ => return None,
    })
}

/// Resolve a `$` link to a global definition.
fn resolve_definition(head: &str, base: &str) -> StrResult<String> {
    let mut parts = head.trim_start_matches('$').split('.').peekable();
    let mut focus = &LIBRARY.global;
    let mut category = None;

    while let Some(name) = parts.peek() {
        if category.is_none() {
            category = focus.scope().get(name).and_then(Binding::category);
        }
        let Ok(module) = get_module(focus, name) else { break };
        focus = module;
        parts.next();
    }

    let Some(category) = category else { bail!("{head} has no category") };

    let name = parts.next().ok_or("link is missing first part")?;
    let value = focus.field(name, ())?;

    // Handle grouped functions.
    if let Some(group) = GROUPS.iter().find(|group| {
        group.category == category && group.filter.iter().any(|func| func == name)
    }) {
        let mut route = format!(
            "{}reference/{}/{}/#functions-{}",
            base,
            group.category.name(),
            group.name,
            name
        );
        if let Some(param) = parts.next() {
            route.push('-');
            route.push_str(param);
        }
        return Ok(route);
    }

    let mut route = format!("{}reference/{}/{name}", base, category.name());
    if let Some(next) = parts.next() {
        if let Ok(field) = value.field(next, ()) {
            route.push_str("/#definitions-");
            route.push_str(next);
            if let Some(next) = parts.next()
                && field.cast::<Func>().is_ok_and(|func| func.param(next).is_some())
            {
                route.push('-');
                route.push_str(next);
            }
        } else if value
            .clone()
            .cast::<Func>()
            .is_ok_and(|func| func.param(next).is_some())
        {
            route.push_str("/#parameters-");
            route.push_str(next);
        } else {
            bail!("field {next} not found");
        }
    }

    Ok(route)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function() {
        assert_eq!(
            resolve_definition("$figure", "/"),
            Ok("/reference/model/figure".into())
        );
        assert_eq!(
            resolve_definition("$figure.body", "/"),
            Ok("/reference/model/figure/#parameters-body".into())
        );
        assert_eq!(
            resolve_definition("$figure.caption", "/"),
            Ok("/reference/model/figure/#definitions-caption".into())
        );
        assert_eq!(
            resolve_definition("$figure.caption.position", "/"),
            Ok("/reference/model/figure/#definitions-caption-position".into())
        );
    }

    #[test]
    fn test_function_definition() {
        assert_eq!(
            resolve_definition("$outline", "/"),
            Ok("/reference/model/outline".into())
        );
        assert_eq!(
            resolve_definition("$outline.title", "/"),
            Ok("/reference/model/outline/#parameters-title".into())
        );

        assert_eq!(
            resolve_definition("$outline.entry", "/"),
            Ok("/reference/model/outline/#definitions-entry".into())
        );
        assert_eq!(
            resolve_definition("$outline.entry.fill", "/"),
            Ok("/reference/model/outline/#definitions-entry-fill".into())
        );
    }

    #[test]
    fn test_function_definition_definition() {
        assert_eq!(
            resolve_definition("$outline.entry.indented", "/"),
            Ok("/reference/model/outline/#definitions-entry-definitions-indented".into())
        );
        assert_eq!(
            resolve_definition("$outline.entry.indented.prefix", "/"),
            Ok("/reference/model/outline/#definitions-entry-definitions-indented-prefix"
                .into())
        );
    }

    #[test]
    fn test_type() {
        assert_eq!(
            resolve_definition("$array", "/"),
            Ok("/reference/foundations/array".into())
        );
        assert_eq!(
            resolve_definition("$array.at", "/"),
            Ok("/reference/foundations/array/#definitions-at".into())
        );
        assert_eq!(
            resolve_definition("$array.at.index", "/"),
            Ok("/reference/foundations/array/#definitions-at-index".into())
        );
    }

    #[test]
    fn test_type_constructor() {
        assert_eq!(
            resolve_definition("$str.base", "/"),
            Ok("/reference/foundations/str/#constructor-base".into())
        );
        assert_eq!(
            resolve_definition("$tiling.relative", "/"),
            Ok("/reference/visualize/tiling/#constructor-relative".into())
        );
    }

    #[test]
    fn test_group() {
        assert_eq!(
            resolve_definition("$calc.abs", "/"),
            Ok("/reference/foundations/calc/#functions-abs".into())
        );
        assert_eq!(
            resolve_definition("$calc.pow.exponent", "/"),
            Ok("/reference/foundations/calc/#functions-pow-exponent".into())
        );
    }

    #[test]
    fn test_redundant_field() {
        assert_eq!(
            resolve_definition("$figure.body.anything", "/"),
            Err("found redundant field anything".into())
        );
    }
}
