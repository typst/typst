--- tags-grouping ---
// Test how grouping rules handle tags at their edges. To observe this scenario,
// we can in principle have a link at the start or end of a paragraph and see
// whether the two nest correctly.
//
// Unfortunately, there isn't really a way to test end tags from Typst code.
// Hence, we're simulating it with metadata here. Note that this tests for
// slightly different, but even a bit more complex behavior since each metadata
// has its own start and end tags. Effectively, we are enforcing that not just
// the trailing end tag is kept in the paragraph grouping, but also that
// start/end combos before it are kept, too.

// Hide everything ... we don't need a reference image.
#set text(size: 0pt)
#show: hide
#show ref: [Ref]

#let case(body, output) = context {
  // Get a unique key for the case.
  let key = here()
  let tagged(s, it) = {
    metadata((key, "<" + s + ">"))
    it
    metadata((key, "</" + s + ">"))
  }

  // Note: This only works for locatable elements because otherwise the
  // metadata tags won't be sandwiched by other tags and not forced into the
  // paragraph grouping.
  show par: tagged.with("p")
  show link: tagged.with("a")
  show ref: tagged.with("ref")
  body

  context test(
    // Finds only metadata keyed by `key`, which is unique for this case.
    query(metadata)
      .filter(e => e.value.first() == key)
      .map(e => e.value.last())
      .join(),
    output
  )
}

// Both link and ref are contained in the paragraph.
#case(
  [@ref #link("A")[A]],
  "<p><ref></ref><a></a></p>"
)

// When there's a trailing space, that's okay.
#case(
  [@ref #link("A")[A ]],
  "<p><ref></ref><a></a></p>"
)

// Both link and ref are contained in the paragraph.
#case(
  [#link("A")[A] @ref],
  "<p><a></a><ref></ref></p>"
)

// When there's a leading space, that's okay.
#case(
  [#link("A")[ A] @ref],
  "<p><a></a><ref></ref></p>"
)

// When there's only a link, it will surround the paragraph.
#case(
  link("A")[A],
  "<a><p></p></a>"
)

--- tags-textual ---
// Ensure that tags and spaces aren't reordered in textual grouping.
A#metadata(none)<a> #metadata(none)<b>#box[B]

#context assert(
  locate(<a>).position().x + 1pt < locate(<b>).position().x
)
