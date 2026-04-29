// Definies utilites and basic components.

#import "system.typ": colors, sizes, asset-base

// Defines the current section title.
#let title-state = state("title")

// Creates per-web-page unique numbers for tooltips.
#let tooltip-counter = counter("tooltip")

// Attaches a label to some content without having to enter markup mode.
#let labelled(c, l) = [#c#l]

// Converts a string to title case (fairly naively).
#let title-case(string) = {
  string
    .split(regex("[\\s-]"))
    .map(word => {
      let first = word.first()
      upper(first) + lower(word.slice(first.len()))
    })
    .join(" ", default: "")
}

// Extracts the first paragraph of a body string or content, naively.
//
// You are not supposed to treat content like this, but I don't have a better
// way to do this.
#let oneliner(body) = {
  if type(body) == str {
    body.split("\n\n").first().replace("\n", " ")
  } else if body.has("children") {
    body.children.split(parbreak()).first().join()
  } else {
    body
  }
}

// Casts a function-like value to a function (e.g. a callable symbol).
//
// A bit of a hack since there is no good native way.
#let to-func(it) = outline(indent: it).indent

// Builds a CSS class string.
//
// - Positional arguments that are not `none` are added
// - Named arguments are filtered for those who map to true
//
// The result is joined with a space. This works just like the typical
// `classnames` utility in JS.
#let classnames(..args) = {
  args.pos().filter(v => v != none)
  args.named().pairs().filter(((k, v)) => v).map(((k, _)) => k)
}.join(" ")

// Builds a CSS inline style string from named arguments with string values.
#let inline-style(..args) = {
  assert.eq(args.pos().len(), 0, message: "style properties must be named")
  args.named().pairs().map(((k, v)) => k + ": " + v).join("; ")
}

// Adds extra heading nesting depth.
#let heading-offset(offset, body) = context {
  set heading(offset: heading.offset + offset)
  body
}

// Adds extra heading nesting depth in paged export.
#let paged-heading-offset(offset, body) = context {
  set heading(offset: heading.offset + offset) if target() == "paged"
  body
}

#let is-short-state = state("is-short", false)

// Takes two versions: A long one and a short one.
//
// Displays the long version by default, but the short one in content wrapped
// in `with-short-version`. Typst's plain text mechanism will also extract the
// short version (which is e.g. useful for usage with the PDF outline).
#let short-or-long(short, long) = {
  {
    // This is only for the plain text mechanism. Since it doesn't use proper
    // realization, it will ignore both the show rule and the full context block
    // below, so it will extract exactly the `short` version.
    show block: none
    block(short)
  }
  context if is-short-state.get() { short } else { long }
}

// For all `short-or-long` content in `body`, prefers the short version.
#let with-short-versions(body) = {
  is-short-state.update(true)
  body
  is-short-state.update(false)
}

// Contraints content to a maximum width and/or height.
#let constrain(width: none, height: none, body) = layout(size => block(
  ..if width != none { (width: calc.min(size.width, width)) },
  ..if height != none { (height: calc.min(size.height, height)) },
  body,
))

// Displays text a bit smaller and in gray.
#let small(body) = context if target() == "paged" {
  text(size: sizes.small, fill: colors.dark-gray.shade-05, body)
} else {
  html.small(body)
}

// Displays a small icon with a size class, name, and alt text.
//
// An example usage would be `icon(16, "close", "Close")` to load `16-close.svg`
// with `alt="Close"`.
#let icon(size, name, alt) = context {
  let filename = str(size) + "-" + name + ".svg"
  if target() == "paged" {
    // Scale roughly with size class, with 16 = 1em.
    let height = 1em * size / 16
    box(height: 0.5 * height, move(
      dy: -0.3 * height,
      image("../assets/" + filename, height: height),
    ))
  } else {
    html.img(
      src: asset-base + filename,
      width: size,
      height: size,
      alt: alt,
    )
  }
}

// TODO: What's the difference compared to `icon`?
#let use-icon(size, name, alt) = html.elem(
  "svg",
  attrs: {
    let s = str(size)
    (
      width: s,
      height: s,
      viewBox: "0 0 " + s + " " + s,
      preserveAspectRatio: "xMidYMid meet",
      role: "img",
    )
  },
  {
    html.elem("title", alt)
    html.elem("use", attrs: (
      "href": asset-base + str(size) + "-" + name + ".svg" + "#icon",
    ))
  },
)

// Applies a tooltip to some content in HTML export. No-op in paged export.
#let with-tooltip(body, description) = context {
  // Paged target does not support tooltips.
  if target() == "paged" { return body }

  // TODO: The tooltip counter could be replaced with a label-based mechanism.
  let id = "tooltip-" + str(tooltip-counter.get().first())
  html.span(aria-describedby: id, body)
  html.div(class: "tooltip-context", {
    use-icon(12, "tooltip", "Question mark")
    html.div(
      id: id,
      role: "tooltip",
      tabindex: -1,
      aria-hidden: true,
      description,
    )
  })
  tooltip-counter.step()
}

// Displays a foldable details block or just the content in paged export.
#let folding-details(title: none, open: false, body) = context {
  if target() == "paged" { return body }
  html.details(class: "folding-example", open: open, {
    html.summary({
      html.img(
        src: asset-base + "16-chevron-right.svg",
        alt: "",
        width: 16,
        height: 16,
      )
      title
    })
    body
  })
}

// Displays a string containing plain text and backticks as text with inline raw
// blocks.
#let text-with-code(t) = context {
  t
    .split("`")
    .enumerate()
    .map(((i, s)) => {
      let even = calc.even(i)
      if target() == "paged" {
        if even { s } else { raw(s) }
      } else {
        if even { html.span(s) } else { html.code(s) }
      }
    })
    .join()
}

// Displays a deprecation info, if any.
#let deprecation(info) = {
  if info == none { return }

  let body = {
    text-with-code(info.message)
    if info.until != none {
      [; it will be removed in Typst #info.until]
    }
  }

  context if target() == "paged" {
    small(icon(16, "warn", "Warning") + [ ] + body)
  } else {
    html.small(class: "deprecation", {
      html.div(use-icon(16, "warn", "Warning"))
      html.span(body)
    })
  }
}

// Displays a search box.
//
// Only support in the web output.
#let search-box(..props) = html.div(class: "search", {
  icon(16, "search-gray", "Search")
  html.input(type: "search", ..props)
})

// Displays an informational callout.
#let info(body) = context {
  if target() == "paged" {
    block(
      stroke: colors.blue.shade-50 + 0.5pt,
      fill: colors.blue.shade-05,
      radius: 0.5em,
      inset: 1em,
      {
        block(
          sticky: true,
          strong(delta: 0, text(
            fill: colors.blue.shade-80,
            size: sizes.small,
          )[INFO]),
        )
        body
      },
    )
  } else {
    html.div(class: "info-box", body)
  }
}

// Displays a combination of a summary and a body.
#let details(summary, body) = context {
  if target() == "paged" {
    emph(summary)
    parbreak()
    body
  } else {
    html.details({
      html.summary(summary)
      body
    })
  }
}

// Displays a keyboard shortcut. Takes a string rather than content.
#let kbd(shortcut) = context {
  if target() == "paged" {
    set text(fill: colors.light-gray.shade-70)
    let r = 3pt
    box(
      fill: colors.light-gray.shade-05,
      stroke: colors.light-gray.shade-30 + 0.75pt,
      outset: (y: r),
      inset: (x: r),
      radius: r,
      raw(shortcut),
    )
  } else {
    html.kbd(shortcut)
  }
}
