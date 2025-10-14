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
