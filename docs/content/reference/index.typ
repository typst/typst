#import "../../components/index.typ": docs-chapter, paged-heading-offset

#docs-chapter(
  title: "Reference",
  route: "/reference",
  description: "The Typst reference is a systematic and comprehensive guide to the Typst typesetting language.",
  introduction: true,
)[
  This reference documentation is a comprehensive guide to all of Typst's syntax, concepts, types, and functions. If you are completely new to Typst, we recommend starting with the @tutorial[tutorial] and then coming back to the reference to learn more about Typst's features as you need them.

  This is where you will find official documentation for the Typst programming language (its @reference:syntax[syntax], @reference:styling[styling capabilities], @reference:scripting[scripting tools], and @reference:context[dynamic features]), its standard library, and its export-specific features.

  = Version <version>

  This reference is specific to Typst #sys.version. In the CLI, you can check your Typst version with `typst --version` and update to the most recent version with `typst update`. On the web app, the Typst version used for a specific project can be selected in the Settings panel. The Typst version is sometimes refered to as the "compiler version."
]

#show: paged-heading-offset.with(1)
#include "language/index.typ"
#include "library/index.typ"
#include "export/index.typ"
