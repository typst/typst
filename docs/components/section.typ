#import "base.typ": (
  classnames, heading-offset, icon, labelled, short-or-long, title-state,
  tooltip-counter,
)
#import "linking.typ": register-def
#import "nav.typ": nav-breadcrumbs, nav-folding, nav-on-this-page, nav-prev-next
#import "search.typ": register-index-item
#import "styling.typ": prose-styling, styling

// One section in the paged documentation.
#let paged-section(
  title-fmt: none,
  title-content: none,
  def-title: none,
  introduction: none,
  def-target: none,
  body,
) = {
  title-state.update(none)

  // New section starts on a new page.
  pagebreak(weak: true)

  // Section heading.
  {
    show heading: it => {
      let loc = it.location()

      // All sections referenceable under their definition target. We want to
      // link to the section heading.
      if def-target != none {
        register-def(def-target, loc, title: def-title)
      }

      // Update what's shown in the page header.
      title-state.update({
        if it.numbering != none {
          counter(heading).display(it.numbering, at: loc)
        }
        [ ]
        title-fmt
      })

      it
    }
    heading(title-content)
  }

  let offset = 1

  // If requested, add an invisible introduction heading and increase the
  // heading nesting depth by one extra.
  if introduction {
    show heading: none
    heading(depth: 2, numbering: none)[Introduction]
    offset += 1
  }

  heading-offset(offset, body)
}

// One section (page) in the website documentation.
#let html-section(
  title: none,
  title-content: none,
  has-summary: none,
  def-title: none,
  route: none,
  def-target: none,
  class: none,
  nav-buttons: none,
  kind: none,
  keywords: none,
  description: none,
  body,
) = context {
  let depth = calc.max(
    route.split("/").filter(seg => seg.len() != 0).len(),
    1,
  )

  let route = stdx.config.content-base + route.trim("/")
  if not route.ends-with("/") {
    route += "/"
  }

  if def-target != none {
    register-def(def-target, route, title: def-title)
  }

  // Reset the tooltip counter for each page.
  tooltip-counter.update((0,))

  document(route + "index.html", title: title, html.html({
    // The index item is placed here so that all body text of this HTML document
    // is considered part of this index item.
    register-index-item(
      kind: kind,
      title: title,
      dest: route,
      keywords: keywords,
    )

    // This metadata is used for the section navigation.
    labelled(
      metadata((route: route, title: title, depth: depth)),
      <metadata-page>,
    )

    html.head({
      html.meta(charset: "utf-8")
      html.meta(
        name: "viewport",
        content: "width=device-width, initial-scale=1",
      )
      html.meta(name: "description", content: description)
      html.meta(name: "theme-color", content: "#239dad")
      html.meta(name: "robots", content: "noindex")
      html.link(href: stdx.config.asset-base + "base.css", rel: "stylesheet")
      html.link(href: stdx.config.asset-base + "docs.css", rel: "stylesheet")
      html.script(type: "module", src: stdx.config.asset-base + "docs.js")
      html.title(title + " - Typst Documentation")
      // TODO: More to come here
    })

    html.body(class: classnames("docs", class), {
      html.header(class: "w695", {
        html.button(
          class: "hamburger",
          icon(16, "hamburger-dark", "Open navigation"),
        )
      })
      html.div(class: "main-grid", {
        nav-folding(route)
        context {
          html.main({
            nav-breadcrumbs(route)
            labelled(std.title(title-content), if has-summary { <summary> })
            body
            nav-buttons
          })
          nav-on-this-page(here())
        }
      })
    })
  }))
}

// One section in the documentation.
//
// Corresponds to one page in the HTML version and to one subsection in paged
// export. The depth of the section in PDF export varies. For instance, "Syntax"
// is a section at depth 2, but "Float" is also a section, at depth 3.
#let docs-section(
  // The plain-text title of the section.
  title: none,
  // The richly formatted title of the section.
  title-fmt: auto,
  // An optional subtitle for the section, displayed within the heading.
  // This is, for instance, used for the "Element" suffix.
  subtitle: none,
  // Whether the page outline should have a "Summary" entry.
  has-summary: false,
  // Whether the section is an introductory section of a larger one. If so, it
  // will get a hidden "Introduction" heading in the PDF outline and the
  // headings in its body skip an extra level.
  introduction: false,

  // The URL path of the section in the web version. Will be amended to the docs
  // base path.
  route: none,
  // The definition target for the section. See `linking.typ` for more
  // information.
  def-target: none,

  // CSS class(es) to apply to the section's HTML page.
  class: none,
  // The navigation buttons at the bottom of the HTML page.
  nav-buttons: nav-prev-next(),

  // The kind of section. This is displayed in the search result for the page.
  kind: none,
  // Keywords for the page. The page can be found in search with these.
  keywords: (),
  // The plain-text description of the HTML page.
  description: none,

  // The content of the section.
  body,
) = {
  assert.ne(title, none, message: "title is required")
  assert.eq(type(title), str, message: "title must be a string")
  assert.ne(route, none, message: "route is required")
  assert.ne(def-target, none, message: "definition target is required")
  assert.ne(description, none, message: "description is required")
  assert.eq(type(description), str, message: "description must be a string")
  assert.ne(kind, none, message: "kind is required")
  assert.eq(type(keywords), array, message: "keywords must be an array")
  assert(route.starts-with("/"), message: "route must start with slash")

  if title-fmt == auto {
    title-fmt = title
  }

  // The title under which the definition will be referenced. If the target is a
  // label, we use the section title. If the target is an std value, we use
  // the default (monospace raw of the target value's path in std).
  let def-title = if type(def-target) == label { title } else { auto }
  let title-content = short-or-long(title, title-fmt + subtitle)

  context if target() == "paged" {
    paged-section(
      title-fmt: title-fmt,
      title-content: title-content,
      def-title: def-title,
      introduction: introduction,
      def-target: def-target,
      body,
    )
  } else {
    html-section(
      title: title,
      title-content: title-content,
      has-summary: has-summary,
      def-title: def-title,
      route: route,
      def-target: def-target,
      class: class,
      nav-buttons: nav-buttons,
      kind: kind,
      keywords: keywords,
      description: description,
      body,
    )
  }
}

// A section of kind "Chapter".
//
// Receives an automatic definition target based on the `route` (where slashes
// are replaced with `:`) and applies prose styling to the body content.
//
// This is used for prose sections like in the tutorial, language reference, and
// guides.
#let docs-chapter(
  route: none,
  def-target: none,
  ..args,
  body,
) = {
  assert.ne(route, none, message: "route is required")

  if def-target == none {
    def-target = label(route.trim("/", at: start).replace("/", ":"))
  }

  docs-section(
    route: route,
    def-target: def-target,
    ..args,
    kind: "Chapter",
    prose-styling(body, base-target: def-target),
  )
}

// An outline for a single section in the docs.
#let section-outline(title: auto, label: none) = {
  assert.ne(label, none, message: "label is required")
  labelled(heading(title, outlined: false), label)

  context if target() == "html" {
    let before = query(selector(<metadata-page>).before(here()))
    let after = query(selector(<metadata-page>).after(here()))
    let min = before.last().value.depth
    let i = 0
    for m in after {
      if m.value.depth == min + 1 {
        list.item(link(m.value.route, m.value.title))
      } else if m.value.depth <= min {
        break
      }
    }
  } else {
    let level = 2
    let candidates = heading.where(level: level)
    let parents = heading.where(level: level - 1)
    outline(
      title: none,
      target: candidates.after(here()).before(parents.after(here())),
      indent: 0pt,
      depth: level,
    )
  }
}
