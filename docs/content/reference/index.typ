#import "../../components/index.typ": (
  docs-chapter, modifier-list, paged-heading-offset, ty-pill,
)

#docs-chapter(
  title: "Reference",
  route: "/reference",
  description: "The Typst reference is a systematic and comprehensive guide to the Typst typesetting language.",
  introduction: true,
  class: "reference-index",
)[
  This reference documentation is a comprehensive guide to all of Typst's syntax, concepts, functions, types, and other definitions. Use the reference to answer specific questions about Typst and to broaden your understanding of the available features.

  If you are completely new to Typst, we recommend starting with the @tutorial[tutorial] and then coming back to the reference to learn more about Typst's features as you need them.

  = Language <language>
  The reference starts by covering fundamentals of the Typst language. First, we give an overview of @reference:syntax[Typst's syntax.] The following sections cover core concepts central to the Typst language such as @reference:styling[styling documents,] using @reference:scripting[Typst's scripting capabilities,] and @reference:context[reasoning about the contents of your document.]

  = Library <library>
  #context if target() == "paged" [
    // The PDF outline does not contain the part labels, so we have to tell the reader where each section starts.
    Starting with @reference:foundations, the reference includes sections
  ] else [
    The second part includes sections
  ] on all functions, types, and other definitions provided by the _standard library_ of the Typst language.

  The definition sections are grouped by topic. For example, if you would like to explore all tools Typst provides to adjust where elements land on the page, you should start in the @reference:layout section.

  = Export <export>
  Some of the features in Typst only apply to certain output file formats.
  #context if target() == "paged" [
    Starting in the @pdf[PDF] section, you can find chapters for each of the output formats Typst supports. This is where
  ] else [
    In the third part,
  ]
 you find the available format-specific settings and learn what features are available to customize your document for a given format.

  = Reading the reference <reading-the-reference>
  This reference uses a few graphical conventions and labels to let you quickly scan its sections.

  / #ty-pill(
      str,
      linked: false,
    ): These pills indicate that a value is of a particular type. Each type's chapter uses the respective pill as its title. Similar types share a color. For example, all numeric types have the same color.

  / #modifier-list[Element]: Some functions are labelled as elements. This means that they can be used with set and show rules. Some elements can be @locate[located] and used with the @query function. Elements generally produce visible output in the document. You may be using elements even if you are not calling functions, as there is dedicated markup for some elements.

  / #modifier-list[Contextual]: These functions can reason about the contents of your document. They can only be used when _context_ is available, for example through a context block. Refer to the @reference:context section for more information.

  / #modifier-list[Required]: Appears on a function parameter if calling the function without that parameter would result in an error.

  / #modifier-list[Positional]: Appears on a function parameter that is specified without a parameter name and colon. Instead, Typst will use the parameter order to determine which argument is which. Parameters not marked as positional are _named_ parameters.

  / #modifier-list[Variadic]: Appears on function parameters that can be specified multiple times.

  / #modifier-list[Settable]: Appears on function parameters of element functions that can be customized with a set rule.
]

#show: paged-heading-offset.with(1)
#include "language/index.typ"
#include "library/index.typ"
#include "export/index.typ"
