// Navigation components for the HTML version.
//
// Defines
// - `nav-folding`: The collapsible navigation on the left
// - `nav-breadcrumbs`: The docs "path" at the top
// - `nav-on-this-page`: The outline on the right
// - `nav-prev-next`: The navigation buttons on the bottom

#import "base.typ": (
  icon, inline-style, labelled, search-box, with-short-versions,
)

// Registers a section (not a page!) in the nav.
//
// Used for things like "Language" and "Library".
#let nav-separation(depth: 2, title) = labelled(
  metadata((
    title: title,
    depth: depth,
  )),
  <metadata-nav-separation>,
)

// Nav items at a specific depth.
#let nav-items(
  items,
  current,
  is-expanded,
) = html.ul(
  style: inline-style(
    overflow: "visible hidden",
    ..if is-expanded {
      (max-height: "none", pointer-events: "auto", opacity: "1")
    } else {
      (max-height: "0px", pointer-events: "none", opacity: "0")
    },
  ),
  {
    let i = 0
    let len = items.len()
    while i < len {
      let item = items.at(i)
      i += 1
      let k = i
      while i < len and item.depth < items.at(i).depth {
        i += 1
      }

      if "route" in item {
        let has-children = k < i
        let is-current = item.route == current
        let is-expanded = has-children and current.starts-with(item.route)
        html.li(aria-expanded: is-expanded, {
          html.a(
            href: item.route,
            item.title,
            ..if item.route == current { (aria-current: "page") },
          )
          if k < i {
            html.button(icon(16, "chevron-right", "Expand"))
            nav-items(items.slice(k, i), current, is-expanded)
          }
        })
      } else {
        html.li(class: "category", item.title)
      }
    }
  },
)

// The folding navigation on the left of the docs.
#let nav-folding(current) = html.nav(class: "folding", context {
  html.button(class: "close", icon(16, "close", "Close"))
  search-box(id: "docs-search", placeholder: "Search (S)")
  html.ul(id: "search-results", class: "search-results hidden")
  let items = query(selector.or(
    <metadata-page>,
    <metadata-nav-separation>,
  )).map(meta => meta.value)
  nav-items(items, current, true)
})

// Maps from page routes to their titles.
#let title-map() = {
  let pages = query(<metadata-page>)
  pages.map(page => (page.value.route, page.value.title)).to-dict()
}

// The breadcrumbs at the top of the page.
#let nav-breadcrumbs(route) = html.ul(
  class: "breadcrumbs",
  aria-label: "Breadcrumbs",
  context {
    html.li(class: "root", html.a(
      href: sys.inputs.base,
      icon(16, "docs-dark", "Docs"),
    ))
    let titles = title-map()
    for m in route.matches("/") {
      let parent = route.slice(0, m.end)
      let title = titles.at(parent, default: none)
      if title == none or title == "Overview" { continue }
      html.li(aria-hidden: true, icon(16, "chevron-right", ""))
      html.li(html.a(href: parent, title))
    }
  },
)

// Outline items at a specific depth.
#let nav-on-this-page-items(items) = html.ul({
  let i = 0
  let len = items.len()
  while i < len {
    let item = items.at(i)
    i += 1
    let k = i
    while i < len and item.level < items.at(i).level {
      i += 1
    }
    html.li({
      link(item.dest, item.body)
      if k < i {
        let children = items.slice(k, i)
        nav-on-this-page-items(children)
      }
    })
  }
})

// An outline for a single web page, on the right of the page.
#let nav-on-this-page(scope) = context {
  let items = ()

  let titles = query(docs.selector-within(
    selector.and(title, <summary>),
    scope,
  ))
  if titles.len() > 0 {
    let dest = titles.first().location()
    items.push((dest: dest, level: 1, body: [Summary]))
  }

  items += query(docs.selector-within(
    heading.where(outlined: true),
    scope,
  )).map(m => (dest: m.location(), level: m.level, body: m.body))

  if items.len() == 0 { return }

  html.nav(id: "page-overview", {
    strong[On this page]
    with-short-versions(nav-on-this-page-items(items))
  })
}

// The nav buttons on normal pages.
#let nav-button(
  href: none,
  icon: none,
  class: none,
  title: none,
  hint: none,
) = html.a(
  href: href,
  class: "nav-button " + class,
  {
    icon
    html.div({
      html.span(class: "page-title", title)
      html.span(class: "hint", hint)
    })
  },
)

// The big nav buttons on the overview page.
#let big-nav-button(
  href: none,
  icon: none,
  title: none,
  description: none,
) = html.a(
  href: href,
  class: "nav-button",
  {
    icon
    html.strong(title)
    html.p(description)
  },
)

// The navigation buttons at the bottom of the page.
#let nav-prev-next() = html.div(class: "page-end-buttons", context {
  let prev = query(selector(<metadata-page>).before(here()))
  if prev.len() > 1 {
    let (title, route) = prev.at(-2).value
    nav-button(
      icon: icon(16, "chevron-right", "←"),
      class: "previous",
      hint: [Previous page],
      href: route,
      title: title,
    )
  }
  let next = query(selector(<metadata-page>).after(here()))
  if next.len() > 0 {
    let (title, route) = next.first().value
    nav-button(
      icon: icon(16, "chevron-right", "→"),
      class: "next",
      hint: [Next page],
      href: route,
      title: title,
    )
  }
})
