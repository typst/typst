#import "utils.typ": *

= Scripting <scripting>
- Plugins (thanks to #gh("astrale-sharp") and #gh("arnaudgolfouse"))
  - Typst can now load @plugin[plugins] that are compiled to WebAssembly
  - Anything that can be compiled to WebAssembly can thus be loaded as a plugin
  - These plugins are fully encapsulated (no access to file system or network)
  - Plugins can be shipped as part of @reference:scripting:packages[packages]
  - Plugins work just the same in the web app
- Types are now first-class values *(Breaking change)*
  - A @type[type] is now itself a value
  - Some types can be called like functions (those that have a constructor), e.g. @int and @str
  - Type checks are now of the form `{type(10) == int}` instead of the old `{type(10) == "integer"}`. Compatibility with the old way will remain for a while to give package authors time to upgrade, but it will be removed at some point.
  - Methods are now syntax sugar for calling a function scoped to a type, meaning that `{"hello".len()}` is equivalent to `{str.len("hello")}`
- Added support for @reference:scripting:modules[`import`] renaming with `as`
- Added a @duration type
- Added support for @cbor[CBOR] encoding and decoding
- Added encoding and decoding functions from and to bytes for data formats: @json.decode, @json.encode, and similar functions for other formats
- Added @array.intersperse function
- Added @str.rev function
- Added `calc.tau` constant
- Made @bytes[bytes] joinable and addable
- Made @array.zip function variadic
- Fixed bug with @eval when the `mode` was set to `{"math"}`
- Fixed bug with @str.ends-with[`ends-with`] function on strings
- Fixed bug with destructuring in combination with break, continue, and return
- Fixed argument types of @calc.cosh[hyperbolic functions], they don't allow angles anymore *(Breaking change)*
- Renamed some color methods: `rgba` becomes `to-rgba`, `cmyk` becomes `to-cmyk`, and `luma` becomes `to-luma` *(Breaking change)*

= Export <export>
- Added SVG export (thanks to #gh("Enter-tainer"))
- Fixed bugs with PDF font embedding
- Added support for page labels that reflect the @page.numbering[page numbering] style in the PDF

= Text and Layout <text-and-layout>
- Added @highlight function for highlighting text with a background color
- Added @polygon.regular function for drawing a regular polygon
- Added support for tabs in @raw elements alongside @raw.tab-size[`tab-width`] parameter
- The layout engine now tries to prevent "runts" (final lines consisting of just a single word)
- Added Finnish translations
- Added hyphenation support for Polish
- Improved handling of consecutive smart quotes of different kinds
- Fixed vertical alignments for @page.number-align[`number-align`] argument on page function *(Breaking change)*
- Fixed weak pagebreaks after counter updates
- Fixed missing text in SVG when the text font is set to "New Computer Modern"
- Fixed translations for Chinese
- Fixed crash for empty text in show rule
- Fixed leading spaces when there's a linebreak after a number and a comma
- Fixed placement of floating elements in columns and other containers
- Fixed sizing of block containing just a single box

= Math <math>
- Added support for @math.mat.augment[augmented matrices]
- Removed support for automatic matching of fences like `|` and `||` as there were too many false positives. You can use functions like @math.abs[`abs`] or @math.norm[`norm`] or an explicit @math.lr[`lr`] call instead. *(Breaking change)*
- Fixed spacing after number with decimal point in math
- Fixed bug with primes in subscript
- Fixed weak spacing
- Fixed crash when text within math contains a newline

= Tooling and Diagnostics <tooling-and-diagnostics>
- Added hints when trying to call a function stored in a dictionary without extra parentheses
- Fixed hint when referencing an equation without numbering
- Added more details to some diagnostics (e.g. when SVG decoding fails)

= Command line interface <command-line-interface>
- Added `typst update` command for self-updating the CLI (thanks to #gh("jimvdl"))
- Added download progress indicator for packages and updates
- Added `--format` argument to explicitly specify the output format
- The CLI now respects proxy configuration through environment variables and has a new `--cert` option for setting a custom CA certificate
- Fixed crash when field wasn't present and `--one` is passed to `typst query`

= Miscellaneous Improvements <miscellaneous-improvements>
- Added @guides:page-setup[Page Setup Guide]
- Added @figure.caption function that can be used for simpler figure customization (*Breaking change* because `it.caption` now renders the full caption with supplement in figure show rules and manual outlines)
- Moved `caption-pos` argument to `figure.caption` function and renamed it to `position` *(Breaking change)*
- Added @figure.caption.separator[`separator`] argument to `figure.caption` function
- Added support for combination of and/or and before/after @selector[selectors]
- Packages can now specify a #link("https://github.com/typst/packages#package-format")[minimum compiler version] they require to work
- Fixed parser bug where method calls could be moved onto their own line for `[#let]` expressions in markup *(Breaking change)*
- Fixed bugs in sentence and title case conversion for bibliographies
- Fixed supplements for alphanumeric and author-title bibliography styles
- Fixed off-by-one error in APA bibliography style

= Development <development>
- Made `Span` and `FileId` more type-safe so that all error conditions must be handled by `World` implementors
