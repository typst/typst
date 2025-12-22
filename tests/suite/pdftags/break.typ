--- break-tags-nested-parbreak pdftags ---
#let target = "tel:123"

Start of the first paragraph #link(target)[
  #quote[
    Part of the first paragraph.

    Start of the second paragraph
  ]
] Part of the second paragraph.

--- break-tags-nested-parbreak-with-nested-groups pdftags ---
#let target = "tel:123"

Start of the first paragraph #link(target)[
  `group before`
  #quote[
    `group before`
    Part of the first paragraph.

    Start of the second paragraph
  ]
] Part of the second paragraph.

--- issue-7020-break-tags pdftags ---
Foo #quote($$ + parbreak())

--- issue-7257-break-tags-show-par-none pdftags ---
#show par: none
#show heading: v(0pt) + [A]
// Error: 3-6 internal error: tags weren't properly closed (occurred at crates/typst-pdf/src/tags/tree/build.rs:185:9)
// Hint: 3-6 please report this as a bug
#[= A]B
