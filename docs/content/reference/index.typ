#import "../../components/index.typ": docs-chapter, paged-heading-offset

#docs-chapter(
  title: "Reference",
  route: "/reference",
  description: "The Typst reference is a systematic and comprehensive guide to the Typst typesetting language.",
  introduction: true,
)[
  This reference documentation is a comprehensive guide to all of Typst's syntax, concepts, types, and functions. If you are completely new to Typst, we recommend starting with the @tutorial[tutorial] and then coming back to the reference to learn more about Typst's features as you need them.

  = Language <language>
  The reference starts with a language part that gives an overview over @reference:syntax[Typst's syntax] and contains information about concepts involved in @reference:styling[styling documents,] using @reference:scripting[Typst's scripting capabilities.]

  = Functions <functions>
  The second part includes chapters on all functions used to insert, style, transform, and layout content in Typst documents. Each function is documented with a description of its purpose, a list of its parameters, and examples of how to use it.

  The final part of the reference explains all functions that are used within Typst's code mode to manipulate and transform data. Just as in the previous part, each function is documented with a description of its purpose, a list of its parameters, and examples of how to use it.
]

#show: paged-heading-offset.with(1)
#include "language/index.typ"
#include "library/index.typ"
#include "export/index.typ"
