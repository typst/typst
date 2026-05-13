// Built-in footnotes in HTML do not support a custom head (which we write).
// Also, we want to have footnotes scoped to small excerpts of content. This
// minimal footnote implementation is heavily based on the built-in one, but
// adds this extra feature.

// Creates a listing of all footnotes that are descendants of the element
// with the location `scope`.
#let footnote-container(scope) = context {
  let notes = query(stdx.selector-within(footnote, scope))
  if notes.len() == 0 { return }
  html.elem("section", attrs: (role: "doc-endnotes"), {
    for note in notes {
      let num = counter(footnote).display(note.numbering, at: note.location())
      enum.item({
        html.elem(
          "sup",
          attrs: (role: "doc-backlink"),
          link(note.location())[#num],
        )
        note.body
      })
    }
  })
}

// Can be applied with `show footnote`.
#let footnote-rule(it) = context {
  h(0pt, weak: true)
  // Would be a bit nicer to link to the `li` element, but this is good enough.
  let dest = locate(link.where(dest: it.location()))
  let nums = counter(footnote).display(it.numbering, at: it.location())
  html.elem("sup", attrs: (role: "doc-noteref"), link(dest, nums))
}

// Wraps content and creates a footnote listing at the bottom of it.
#let with-footnotes(body) = context {
  // Reset the footnote counter.
  counter(footnote).update(())
  body
  // Create a footnote container that lists all footnotes within this context
  // block. (That's what the `here()` refers to.)
  footnote-container(here())
}
