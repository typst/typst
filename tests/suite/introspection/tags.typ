--- tags-grouping render ---
// Test how grouping rules handle tags at their edges. To observe this scenario,
// we have a link at the start or end of a paragraph and see whether the two
// nest correctly.
//
// The tag nesting check is done as a custom check as, unfortunately, there isn't
// really a way to test end tags from Typst code.
//
// On top that we are checking that trailing start/end combos tags after the
// grouping, but before a tag that is matched in the group are also kept in the
// grouping. This we can check from within Typst code via show rules with
// metadata, which is done below.

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

// Because `par` is not currently locatable, we need another way to ensure
// tags are generated around it. This can be removed once par is locatable.
#show par: quote
#show quote: it => it.body

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
  "<p><a></a></p>"
)

--- tags-textual render ---
// Ensure that tags and spaces aren't reordered in textual grouping.
A#metadata(none)<a> #metadata(none)<b>#box[B]

#context assert(
  locate(<a>).position().x + 1pt < locate(<b>).position().x
)
