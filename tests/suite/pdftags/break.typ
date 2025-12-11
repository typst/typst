--- break-tags-nested-parbreak pdftags nopdfua ---
#let target = "tel:123"

Start of the first paragraph #link(target)[
  #quote[
    Part of the first paragraph.

    Start of the second paragraph
  ]
] Part of the second paragraph.

--- break-tags-nested-parbreak-with-nested-groups pdftags nopdfua ---
#let target = "tel:123"

Start of the first paragraph #link(target)[
  `group before`
  #quote[
    `group before`
    Part of the first paragraph.

    Start of the second paragraph
  ]
] Part of the second paragraph.

--- break-tags-issue-7020 pdftags nopdfua ---
Foo #quote($$ + parbreak())

--- issue-7257-break-tags-show-par-none pdftags ---
#show par: none
#show heading: v(0pt) + [A]
// Error: 3-6 internal error: tags weren't properly closed (occurred at crates/typst-pdf/src/tags/tree/build.rs:187:9)
// Hint: 3-6 please report this as a bug
#[= A]B
