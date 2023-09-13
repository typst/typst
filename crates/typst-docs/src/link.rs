use typst::diag::{bail, StrResult};
use typst::eval::Func;

use super::{get_module, GROUPS, LIBRARY};

/// Resolve an intra-doc link.
pub fn resolve(link: &str) -> StrResult<String> {
    if link.starts_with('#') || link.starts_with("http") {
        return Ok(link.to_string());
    }

    let (head, tail) = split_link(link)?;
    let mut route = match resolve_known(head) {
        Some(route) => route.into(),
        None => resolve_definition(head)?,
    };

    if !tail.is_empty() {
        route.push('/');
        route.push_str(tail);
    }

    if !route.contains('#') && !route.ends_with('/') {
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
fn resolve_known(head: &str) -> Option<&'static str> {
    Some(match head {
        "$tutorial" => "/docs/tutorial",
        "$reference" => "/docs/reference",
        "$category" => "/docs/reference",
        "$syntax" => "/docs/reference/syntax",
        "$styling" => "/docs/reference/styling",
        "$scripting" => "/docs/reference/scripting",
        "$guides" => "/docs/guides",
        "$packages" => "/docs/packages",
        "$changelog" => "/docs/changelog",
        "$community" => "/docs/community",
        _ => return None,
    })
}

/// Resolve a `$` link to a global definition.
fn resolve_definition(head: &str) -> StrResult<String> {
    let mut parts = head.trim_start_matches('$').split('.').peekable();
    let mut focus = &LIBRARY.global;
    while let Some(m) = parts.peek().and_then(|&name| get_module(focus, name).ok()) {
        focus = m;
        parts.next();
    }

    let name = parts.next().ok_or("link is missing first part")?;
    let value = focus.field(name)?;
    let Some(category) = focus.scope().get_category(name) else {
        bail!("{name} has no category");
    };

    // Handle grouped functions.
    if let Some(group) = GROUPS
        .iter()
        .filter(|_| category == "math")
        .find(|group| group.functions.iter().any(|func| func == name))
    {
        let mut route =
            format!("/docs/reference/math/{}/#functions-{}", group.name, name);
        if let Some(param) = parts.next() {
            route.push('-');
            route.push_str(param);
        }
        return Ok(route);
    }

    let mut route = format!("/docs/reference/{category}/{name}/");
    if let Some(next) = parts.next() {
        if value.field(next).is_ok() {
            route.push_str("#definitions-");
            route.push_str(next);
        } else if value
            .clone()
            .cast::<Func>()
            .map_or(false, |func| func.param(next).is_some())
        {
            route.push_str("#parameters-");
            route.push_str(next);
        } else {
            bail!("field {next} not found");
        }
    }

    Ok(route)
}
