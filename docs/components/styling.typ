// Defines the set of styling rules for
// - docs content at large
// - "prose" content, i.e. hand-written content in chapters or individual
//   element docs (as opposed to componentized non-prose content).

#import "system.typ": colors, fonts, sizes
#import "base.typ": labelled, small, title-state, with-short-versions
#import "example.typ": example, preview, source
#import "footnote.typ": footnote-rule, with-footnotes
#import "linking.typ": def-label, def-metadata, register-def

// The part of the global styling that applies to the paged version.
#let paged-styling(body) = {
  set page(margin: (x: 3cm, y: 2.5cm))
  set text(font: (fonts.body, ..fonts.fallback), size: sizes.body)
  set list(marker: [--])
  set underline(offset: 0.2em)
  set footnote(numbering: "[A]")
  set table.cell(inset: 4pt)

  show title: set text(28pt)

  show heading.where(outlined: true): set heading(numbering: "1.1.1/1.1")
  show heading.where(level: 1): it => pagebreak(weak: true) + it
  show heading.where(level: 1): set text(18pt)
  show heading.where(level: 2): set text(15pt)
  show heading.where(level: 3): set text(13pt)
  show heading.where(level: 4): set text(11pt)
  show heading.where(level: 5): set text(10pt)
  show heading: set block(below: 1em)
  show heading: it => block({
    if it.numbering != none {
      set text(size: sizes.body, weight: "regular")
      let nums = counter(heading).at(it.location())
      let fmt = if nums.len() <= 3 {
        numbering("1.1", ..nums)
      } else {
        numbering("/1.1", ..nums.slice(3))
      }
      box(width: 0pt, align(right, box(width: 3em, {
        small(fmt)
        h(0.5em)
      })))
    }
    it.body
  })

  // Justifying and hyphenating in tables doesn't look great.
  show table: set par(justify: false)

  // Cancel default raw scaling. We reapply it eventually, but because we do so
  // much stuff with raw blocks (also sometimes recreating them), it keeps
  // interfering. Replacing show rules would probably help. We intentionally
  // hardcode 0.8 here instead of using `sizes.mono` because it's unrelated.
  // Here, we cancel the compiler default.
  show raw: set text(size: 1em / 0.8, font: (fonts.mono, ..fonts.fallback))

  // Outline entries pick the short versions of content which makes use of
  // the `short-or-long` component.
  show outline: with-short-versions

  include "preface.typ"

  // Header & footer only start after the preface.
  set page(
    numbering: "1",
    header: {
      counter(footnote).update(())
      context {
        let title = title-state.get()
        if title != none {
          align(right, small(title))
        }
      }
    },
    footer: align(right, small(context counter(page).display())),
  )

  {
    show outline: set heading(bookmarked: true)
    show outline.entry.where(level: 1): set outline.entry(fill: none)
    show outline.entry.where(level: 1): set block(above: 1.2em, below: 0.8em)
    show outline.entry.where(level: 1): set text(
      size: 1.2em,
      weight: "bold",
      fill: colors.dark-gray.shade-60,
    )
    outline(
      depth: 3,
      indent: n => 1.5em * n,
    )
  }

  body
}

// The part of the global styling that applies to the HTML version.
#let html-styling(body) = {
  // Without this, we can't link to headings.
  show heading: it => {
    assert(it.has("label"), message: "headings must be labelled")
    it
  }
  body
}

// Global styling that is applied to the full docs.
#let styling(body) = {
  set document(
    title: "Typst Documentation (Version: " + str(sys.version) + ")",
    author: (
      "Laurenz Mädje",
      "Martin Haug",
      "The Typst Project Developers",
    ),
  )

  show ref: it => {
    let (dest, title) = def-metadata(it.target)
    if it.supplement == auto {
      link(dest, title)
    } else {
      link(dest, it.supplement)
    }
  }

  show raw.where(block: true): source
  show raw.where(lang: "example"): example
  show raw.where(lang: "preview"): preview
  show raw.where(block: false): it => {
    if it.lang == none {
      let t = it.text
      if t.starts-with("{") and t.ends-with("}") {
        return raw(t.slice(1, -1), lang: "typc")
      }
      if t.starts-with("[") and t.ends-with("]") {
        return raw(t.slice(1, -1), lang: "typ")
      }
    }
    // Only now apply scaling to avoid it being applied twice for
    // replaced raw blocks.
    set text(size: sizes.mono)
    it
  }

  context if target() == "paged" {
    paged-styling(body)
  } else {
    html-styling(body)
  }
}

// The part of `prose-styling` that applies to the paged version.
#let paged-prose-styling(body) = {
  set par(justify: true)

  show link: underline
  show link: it => {
    it
    if type(it.dest) == str and it.at("label", default: none) != <_stop> {
      footnote(labelled(link(it.dest), <_stop>))
    } else if type(it.dest) == location {
      let element = query(it.dest).first()
      if element.func() == heading and element.numbering != none {
        super(text(
          fill: colors.dark-gray.shade-05,
          counter(heading).display(
            element.numbering.trim(".", at: end),
            at: element.location(),
          ),
        ))
      }
    }
  }

  body
}

// The part of `prose-styling` that applies to the HTML version.
#let html-prose-styling(body) = {
  show footnote: footnote-rule
  show: with-footnotes
  body
}

// Styling that is applied to hand-written content as opposed to componentized
// elements.
#let prose-styling(
  // A def target (label or std definition) that is prepended to heading labels
  // for referencing. If `body` contains a heading labelled `<syntax>` and the
  // base target is `<heading>`, then, the section can be referenced as
  // `@syntax:heading`.
  base-target: none,
  body,
) = {
  show: rest => if base-target != none {
    show heading: it => {
      register-def(
        label(str(def-label(base-target)) + ":" + str(it.label)),
        it.location(),
        title: it.body,
      )
      it
    }
    rest
  } else {
    rest
  }

  context if target() == "paged" {
    paged-prose-styling(body)
  } else {
    html-prose-styling(body)
  }
}
