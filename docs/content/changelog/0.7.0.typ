#import "utils.typ": *

= Text and Layout <text-and-layout>
- Added support for floating figures through the @figure.placement[`placement`] argument on the figure function
- Added support for arbitrary floating content through the @place.float[`float`] argument on the place function
- Added support for loading `.sublime-syntax` files as highlighting @raw.syntaxes[syntaxes] for raw blocks
- Added support for loading `.tmTheme` files as highlighting @raw.theme[themes] for raw blocks
- Added _bounds_ option to @text.top-edge[`top-edge`] and @text.bottom-edge[`bottom-edge`] arguments of text function for tight bounding boxes
- Removed nonsensical top- and bottom-edge options, e.g. _ascender_ for the bottom edge *(Breaking change)*
- Added @text.script[`script`] argument to text function
- Added @smartquote.alternative[`alternative`] argument to smart quote function
- Added basic i18n for Japanese
- Added hyphenation support for `nb` and `nn` language codes in addition to `no`
- Fixed positioning of @place[placed elements] in containers
- Fixed overflowing containers due to optimized line breaks

= Export <export>
- Greatly improved export of SVG images to PDF. Many thanks to #gh("LaurenzV") for their work on this.
- Added support for the alpha channel of RGBA colors in PDF export
- Fixed a bug with PPI (pixels per inch) for PNG export

= Math <math>
- Improved layout of primes (e.g. in `[$a'_1$]`)
- Improved display of multi-primes (e.g. in `[$a''$]`)
- Improved layout of @math.root[roots]
- Changed relations to show attachments as @math.limits[limits] by default (e.g. in `[$a ->^x b$]`)
- Large operators and delimiters are now always vertically centered
- @box[Boxes] in equations now sit on the baseline instead of being vertically centered by default. Notably, this does not affect @block[blocks] because they are not inline elements.
- Added support for @h.weak[weak spacing]
- Added support for OpenType character variants
- Added support for customizing the @math.class[math class] of content
- Fixed spacing around `.`, `\/`, and `...`
- Fixed spacing between closing delimiters and large operators
- Fixed a bug with math font weight selection
- Symbols and Operators *(Breaking changes)*
  - Added `id`, `im`, and `tr` text @math.op[operators]
  - Renamed `ident` to `equiv` with alias `eq.triple` and removed `ident.strict` in favor of `eq.quad`
  - Renamed `ast.sq` to `ast.square` and `integral.sq` to `integral.square`
  - Renamed `.eqq` modifier to `.equiv` (and `.neqq` to `.nequiv`) for `tilde`, `gt`, `lt`, `prec`, and `succ`
  - Added `emptyset` as alias for `nothing`
  - Added `lt.curly` and `gt.curly` as aliases for `prec` and `succ`
  - Added `aleph`, `beth`, and `gimmel` as alias for `alef`, `bet`, and `gimel`

= Scripting <scripting>
- Fields
  - Added `abs` and `em` field to @length[lengths]
  - Added `ratio` and `length` field to @relative[relative lengths]
  - Added `x` and `y` field to @align.alignment[2d alignments]
  - Added `paint`, `thickness`, `cap`, `join`, `dash`, and `miter-limit` field to @stroke[strokes]
- Accessor and utility methods
  - Added @array.dedup[`dedup`] method to arrays
  - Added `pt`, `mm`, `cm`, and `inches` method to @length[lengths]
  - Added `deg` and `rad` method to @angle[angles]
  - Added `kind`, `hex`, `rgba`, `cmyk`, and `luma` method to @color[colors]
  - Added `axis`, `start`, `end`, and `inv` method to @stack.dir[directions]
  - Added `axis` and `inv` method to @align.alignment[alignments]
  - Added `inv` method to @align.alignment[2d alignments]
  - Added `start` argument to @array.enumerate[`enumerate`] method on arrays
- Added @color.mix function
- Added `mode` and `scope` arguments to @eval function
- Added @bytes type for holding large byte buffers
  - Added @read.encoding[`encoding`] argument to read function to read a file as bytes instead of a string
  - Added @image.decode function for decoding an image directly from a string or bytes
  - Added @bytes function for converting a string or an array of integers to bytes
  - Added @array function for converting bytes to an array of integers
  - Added support for converting bytes to a string with the @str function

= Tooling and Diagnostics <tooling-and-diagnostics>
- Added support for compiler warnings
- Added warning when compilation does not converge within five attempts due to intense use of introspection features
- Added warnings for empty emphasis (`__` and `**`)
- Improved error message for invalid field assignments
- Improved error message after single `#`
- Improved error message when a keyword is used where an identifier is expected
- Fixed parameter autocompletion for functions that are in modules
- Import autocompletion now only shows the latest package version until a colon is typed
- Fixed autocompletion for dictionary key containing a space
- Fixed autocompletion for `for` loops

= Command line interface <command-line-interface>
- Added `typst query` subcommand to execute a @query:command-line-queries[query] on the command line
- The `--root` and `--font-path` arguments cannot appear in front of the command anymore *(Breaking change)*
- Local and cached packages are now stored in directories of the form `[{namespace}/{name}/{version}]` instead of `[{namespace}/{name}-{version}]` *(Breaking change)*
- Now prioritizes explicitly given fonts (via `--font-path`) over system and embedded fonts when both exist
- Fixed `typst watch` not working with some text editors
- Fixed displayed compilation time (now includes export)

= Miscellaneous Improvements <miscellaneous-improvements>
- Added @heading.bookmarked[`bookmarked`] argument to heading to control whether a heading becomes part of the PDF outline
- Added @figure.caption.position[`caption-pos`] argument to control the position of a figure's caption
- Added @metadata function for exposing an arbitrary value to the introspection system
- Fixed that a @state was identified by the pair `(key, init)` instead of just its `key`
- Improved indent logic of @enum[enumerations]. Instead of requiring at least as much indent as the end of the marker, they now require only one more space indent than the start of the marker. As a result, even long markers like `12.` work with just 2 spaces of indent.
- Fixed bug with indent logic of @raw blocks
- Fixed a parsing bug with dictionaries

= Development <development>
- Extracted parser and syntax tree into `typst-syntax` crate
- The `World::today` implementation of Typst dependents may need fixing if they have the same #link("https://github.com/typst/typst/issues/1842")[bug] that the CLI world had
