---
description: |
  Learn what has changed in the latest Typst releases and move your documents
  forward.
---

# Changelog
## Version 0.10.0 (December 4, 2023) { #v0.10.0 }
- Bibliography management
  - Added support for citation collapsing (e.g. `[[1]-[3]]` instead of
    `[[1], [2], [3]]`) if requested by a CSL style
  - Fixed bug where an additional space would appear after a group of citations
  - Fixed link show rules for links in the bibliography
  - Fixed show-set rules on citations
  - Fixed bibliography-related crashes that happened on some systems
  - Corrected name of the GB/T 7714 family of styles from 7114 to 7714
  - Fixed missing title in some bibliography styles
  - Fixed printing of volumes in some styles
  - Fixed delimiter order for contributors in some styles (e.g. APA)
  - Fixed behaviour of alphanumeric style
  - Fixed multiple bugs with GB/T 7714 style
  - Fixed escaping in Hayagriva values
  - Fixed crashes with empty dates in Hayagriva files
  - Fixed bug with spacing around math blocks
  - Fixed title case formatting after verbatim text and apostrophes
  - Page ranges in `.bib` files can now be arbitrary strings
  - Multi-line values in `.bib` files are now parsed correctly
  - Entry keys in `.bib` files now allow more characters
  - Fixed error message for empty dates in `.bib` files
  - Added support for years of lengths other than 4 without leading zeros in
    `.bib` files
  - More LaTeX commands (e.g. for quotes) are now respected in `.bib` files

- Visualization
  - Added support for [patterns]($pattern) as fills and strokes
  - The `alpha` parameter of the [`components`]($color.components) function on
    colors is now a named parameter (**Breaking change**)
  - Added support for the [Oklch]($color.oklch) color space
  - Improved conversions between colors in different color spaces
  - Removed restrictions on [Oklab]($color.oklab) chroma component
  - Fixed [clipping]($block.clip) on blocks and boxes without a stroke
  - Fixed bug with [gradients]($gradient) on math
  - Fixed bug with gradient rotation on text
  - Fixed bug with gradient colors in PDF
  - Fixed relative base of Oklab chroma ratios
  - Fixed Oklab color negation

- Text and Layout
  - CJK text can now be emphasized with the `*` and `_` syntax even when there
    are no spaces
  - Added basic i18n for Greek and Estonian
  - Improved default [figure caption separator]($figure.caption.separator) for
    Chinese, French, and Russian
  - Changed default [figure supplement]($figure.supplement) for Russian to
    short form
  - Fixed [CJK-Latin-spacing]($text.cjk-latin-spacing) before line breaks
    and in [`locate`]($locate) calls
  - Fixed line breaking at the end of links

- Math
  - Added [`mid`]($math.mid) function for scaling a delimiter up to the height
    of the surrounding [`lr`]($math.lr) group
  - The [`op`]($math.op) function can now take any content, not just strings
  - Improved documentation for [math alignment]($category/math/#alignment)
  - Fixed swallowing of trailing comma when a symbol is used in a function-like
    way (e.g. `pi(a,b,)`)

- Scripting
  - Any non-identifier dictionary key is now interpreted as an expression: For
    instance, `{((key): value)}` will create a dictionary with a dynamic key
  - The [`stroke`]($stroke) type now has a constructor that converts a value to
    a stroke or creates one from its parts
  - Added constructor for [`arguments`]($arguments) type
  - Added [`calc.div-euclid`]($calc.div-euclid) and
    [`calc.rem-euclid`]($calc.rem-euclid) functions
  - Fixed equality of [`arguments`]($arguments)
  - Fixed [`repr`]($repr) of [`cmyk`]($color.cmyk) colors
  - Fixed crashes with provided elements like figure captions, outline entries,
    and footnote entries

- Tooling and Diagnostics
  - Show rules that match on their own output now produce an appropriate error
    message instead of a crash (this is a first step, in the future they will
    just work)
  - Too highly or infinitely nested layouts now produce error messages instead
    of crashes
  - Added hints for invalid identifiers
  - Added hint when trying to use a manually constructed footnote or outline
    entry
  - Added missing details to autocompletions for types
  - Improved error message when passing a named argument where a positional one
    is expected
  - Jump from click now works on raw blocks

- Export
  - PDF compilation output is now again fully byte-by-byte reproducible if the
    document's [`date`]($document.date) is set manually
  - Fixed color export in SVG
  - Fixed PDF metadata encoding of multiple [authors]($document.author)

- Command line interface
  - Fixed a major bug where `typst watch` would confuse files and fail to pick
    up updates
  - Fetching of the release metadata in `typst update` now respects proxies
  - Fixed bug with `--open` flag on Windows when the path contains a space
  - The `TYPST_FONT_PATHS` environment variable can now contain multiple paths
    (separated by `;` on Windows and `:` elsewhere)
  - Updated embedded New Computer Modern fonts to version 4.7
  - The watching process doesn't stop anymore when the main file contains
    invalid UTF-8

- Miscellaneous Improvements
  - Parallelized image encoding in PDF export
  - Improved the internal representation of content for improved performance
  - Optimized introspection (query, counter, etc.) performance
  - The [document title]($document.title) can now be arbitrary content instead
    of just a string
  - The [`number-align`]($enum.number-align) parameter on numbered lists now
    also accepts vertical alignments
  - Fixed selectors on [quote]($quote) elements
  - Fixed parsing of `[#return]` expression in markup
  - Fixed bug where inline equations were displayed in equation outlines
  - Fixed potential CRLF issue in [`raw`]($raw) blocks
  - Fixed a bug where Chinese numbering couldn't exceed the number 255

- Development
  - Merged `typst` and `typst-library` and extracted `typst-pdf`, `typst-svg`,
    and `typst-render` into separate crates
  - The Nix flake now includes the git revision when running `typst --version`

<contributors from="v0.9.0" to="v0.10.0" />

## Version 0.9.0 (October 31, 2023) { #v0.9.0 }
- Bibliography management
  - New bibliography engine based on [CSL](https://citationstyles.org/)
    (Citation Style Language). Ships with about 100 commonly used citation
    styles and can load custom `.csl` files.
  - Added new [`form`]($cite.form) argument to the `cite` function to produce
    different forms of citations (e.g. for producing a citation suitable for
    inclusion in prose)
  - The [`cite`]($cite) function now takes only a single label/key instead of
    allowing multiple. Adjacent citations are merged and formatted according to
    the citation style's rules automatically. This works both with the reference
    syntax and explicit calls to the `cite` function. (**Breaking change**)
  - The `cite` function now takes a [label]($label) instead of a string
    (**Breaking change**)
  - Added [`full`]($bibliography.full) argument to bibliography function to
    print the full bibliography even if not all works were cited
  - Bibliography entries can now contain Typst equations (wrapped in `[$..$]`
    just like in markup), this works both for `.yml` and `.bib` bibliographies
  - The hayagriva YAML format was improved. See its
    [changelog](https://github.com/typst/hayagriva/blob/main/CHANGELOG.md) for
    more details. (**Breaking change**)
  - A few bugs with `.bib` file parsing were fixed
  - Removed `brackets` argument of `cite` function in favor of `form`

- Visualization
  - Gradients and colors (thanks to [@Dherse](https://github.com/Dherse))
    - Added support for [gradients]($gradient) on shapes and text
    - Supports linear, radial, and conic gradients
    - Added support for defining colors in more color spaces, including
      [Oklab]($color.oklab), [Linear RGB(A)]($color.linear-rgb),
      [HSL]($color.hsl), and [HSV]($color.hsv)
    - Added [`saturate`]($color.saturate), [`desaturate`]($color.desaturate),
      and [`rotate`]($color.rotate) functions on colors
    - Added [`color.map`]($color/#predefined-color-maps) module with predefined
      color maps that can be used with gradients
    - Rename `kind` function on colors to [`space`]($color.space)
    - Removed `to-rgba`, `to-cmyk`, and `to-luma` functions in favor of a new
      [`components`]($color.components) function
  - Improved rendering of [rectangles]($rect) with corner radius and varying
    stroke widths
  - Added support for properly clipping [boxes]($box.clip) and
    [blocks]($block.clip) with a border radius
  - Added `background` parameter to [`overline`]($overline),
    [`underline`]($underline), and [`strike`]($strike) functions
  - Fixed inaccurate color embedding in PDFs
  - Fixed ICC profile handling for images embedded in PDFs

- Text and Layout
  - Added support for automatically adding proper
    [spacing]($text.cjk-latin-spacing) between CJK and Latin text (enabled by
    default)
  - Added support for automatic adjustment of more CJK punctuation
  - Added [`quote`]($quote) element for inserting inline and block quotes with
    optional attributions
  - Added [`raw.line`]($raw.line) element for customizing the display of
    individual lines of raw text, e.g. to add line numbers while keeping proper
    syntax highlighting
  - Added support for per-side [inset]($table.inset) customization to table
    function
  - Added Hungarian and Romanian translations
  - Added support for Czech hyphenation
  - Added support for setting custom [smart quotes]($smartquote)
  - The default [figure separator]($figure.caption.separator) now reacts to the
    currently set language and region
  - Improved line breaking of links / URLs (especially helpful for
    bibliographies with many URLs)
  - Improved handling of consecutive hyphens in justification algorithm
  - Fixed interaction of justification and hanging indent
  - Fixed a bug with line breaking of short lines without spaces when
    justification is enabled
  - Fixed font fallback for hyphen generated by hyphenation
  - Fixed handling of word joiner and other no-break characters during
    hyphenation
  - Fixed crash when hyphenating after an empty line
  - Fixed line breaking of composite emoji like 🏳️‍🌈
  - Fixed missing text in some SVGs
  - Fixed font fallback in SVGs
  - Fixed behaviour of [`to`]($pagebreak.to) argument on `pagebreak` function
  - Fixed `{set align(..)}` for equations
  - Fixed spacing around [placed]($place) elements
  - Fixed coalescing of [`above`]($block.above) and [`below`]($block.below)
    spacing if given in em units and the font sizes differ
  - Fixed handling of `extent` parameter of [`underline`]($underline),
    [`overline`]($overline), and [`strike`]($strike) functions
  - Fixed crash for [floating placed elements]($place.float) with no specified
    vertical alignment
  - Partially fixed a bug with citations in footnotes

- Math
  - Added `gap` argument for [`vec`]($math.vec.gap), [`mat`]($math.mat.gap), and
    [`cases`]($math.cases.gap) function
  - Added `size` argument for [`abs`]($math.abs), [`norm`]($math.norm),
    [`floor`]($math.floor), [`ceil`]($math.ceil), and [`round`]($math.round)
    functions
  - Added [`reverse`]($math.cases.reverse) parameter to cases function
  - Added support for multinomial coefficients to [`binom`]($math.binom)
    function
  - Removed `rotation` argument on [`cancel`]($math.cancel) function in favor of
    a new and more flexible `angle` argument (**Breaking change**)
  - Added `wide` constant, which inserts twice the spacing of `quad`
  - Added `csch` and `sech` [operators]($math.op)
  - `↼`, `⇀`, `↔`, and `⟷` can now be used as [accents]($math.accent)
  - Added `integral.dash`, `integral.dash.double`, and `integral.slash`
    [symbols]($category/symbols/sym)
  - Added support for specifying negative indices for
    [augmentation]($math.mat.augment) lines to position the line from the back
  - Fixed default color of matrix [augmentation]($math.mat.augment) lines
  - Fixed attachment of primes to inline expressions
  - Math content now respects the text [baseline]($text.baseline) setting

- Performance
  - Fixed a bug related to show rules in templates which would effectively
    disable incremental compilation in affected documents
  - Micro-optimized code in several hot paths, which brings substantial
    performance gains, in particular in incremental compilations
  - Improved incremental parsing, which affects the whole incremental
    compilation pipeline
  - Added support for incremental parsing in the CLI
  - Added support for incremental SVG encoding during PDF export, which greatly
    improves export performance for documents with many SVG

- Tooling and Diagnostics
  - Improved autocompletion for variables that are in-scope
  - Added autocompletion for package imports
  - Added autocompletion for [labels]($label)
  - Added tooltip that shows which variables a function captures (when hovering
    over the equals sign or arrow of the function)
  - Diagnostics are now deduplicated
  - Improved diagnostics when trying to apply unary `+` or `-` to types that
    only support binary `+` and `-`
  - Error messages now state which label or citation key isn't present in the
    document or its bibliography
  - Fixed a bug where function argument parsing errors were shadowed by
    function execution errors (e.g. when trying to call
    [`array.sorted`]($array.sorted) and passing the key function as a positional
    argument instead of a named one).

- Export
  - Added support for configuring the document's creation
    [`date`]($document.date). If the `date` is set to `{auto}` (the default),
    the PDF's creation date will be set to the current date and time.
  - Added support for configuring document [`keywords`]($document.keywords)
  - Generated PDFs now contain PDF document IDs
  - The PDF creator tool metadata now includes the Typst version

- Web app
  - Added version picker to pin a project to an older compiler version
    (with support for Typst 0.6.0+)
  - Fixed desyncs between editor and compiler and improved overall stability
  - The app now continues to highlight the document when typing while the
    document is being compiled

- Command line interface
  - Added support for discovering fonts through fontconfig
  - Now clears the screen instead of resetting the terminal
  - Now automatically picks correct file extension for selected output format
  - Now only regenerates images for changed pages when using `typst watch` with
    PNG or SVG export

- Miscellaneous Improvements
  - Added [`version`]($version) type and `sys.version` constant specifying the
    current compiler version. Can be used to gracefully support multiple
    versions.
  - The U+2212 MINUS SIGN is now used when displaying a numeric value, in the
    [`repr`]($repr) of any numeric value and to replace a normal hyphen in text
    mode when before a digit. This improves, in particular, how negative integer
    values are displayed in math mode.
  - Added support for specifying a default value instead of failing for
    `remove` function in [array]($array.remove) and
    [dictionary]($dictionary.remove)
  - Simplified page setup guide examples
  - Switched the documentation from using the word "hashtag" to the word "hash"
    where appropriate
  - Added support for [`array.zip`]($array.zip) without any further arguments
  - Fixed crash when a plugin tried to read out of bounds memory
  - Fixed crashes when handling infinite [lengths]($length)
  - Fixed introspection (mostly bibliography) bugs due to weak page break close
    to the end of the document

- Development
  - Extracted `typst::ide` into separate `typst_ide` crate
  - Removed a few remaining `'static` bounds on `&dyn World`
  - Removed unnecessary dependency, which reduces the binary size
  - Fixed compilation of `typst` by itself (without `typst-library`)
  - Fixed warnings with Nix flake when using `lib.getExe`

<contributors from="v0.8.0" to="v0.9.0" />

## Version 0.8.0 (September 13, 2023) { #v0.8.0 }
- Scripting
  - Plugins (thanks to [@astrale-sharp](https://github.com/astrale-sharp) and
    [@arnaudgolfouse](https://github.com/arnaudgolfouse))
    - Typst can now load [plugins]($plugin) that are compiled to WebAssembly
    - Anything that can be compiled to WebAssembly can thus be loaded as a
      plugin
    - These plugins are fully encapsulated (no access to file system or network)
    - Plugins can be shipped as part of [packages]($scripting/#packages)
    - Plugins work just the same in the web app
  - Types are now first-class values (**Breaking change**)
    - A [type]($type) is now itself a value
    - Some types can be called like functions (those that have a constructor),
      e.g. [`int`]($int) and [`str`]($str)
    - Type checks are now of the form `{type(10) == int}` instead of the old
      `{type(10) == "integer"}`. [Compatibility]($type/#compatibility) with the
      old way will remain for a while to give package authors time to upgrade,
      but it will be removed at some point.
    - Methods are now syntax sugar for calling a function scoped to a type,
      meaning that `{"hello".len()}` is equivalent to `{str.len("hello")}`
  - Added support for [`import`]($scripting/#modules) renaming with `as`
  - Added a [`duration`]($duration) type
  - Added support for [CBOR]($cbor) encoding and decoding
  - Added encoding and decoding functions from and to bytes for data formats:
    [`json.decode`]($json.decode), [`json.encode`]($json.encode), and similar
    functions for other formats
  - Added [`array.intersperse`]($array.intersperse) function
  - Added [`str.rev`]($str.rev) function
  - Added `calc.tau` constant
  - Made [bytes]($bytes) joinable and addable
  - Made [`array.zip`]($array.zip) function variadic
  - Fixed bug with [`eval`]($eval) when the `mode` was set to `{"math"}`
  - Fixed bug with [`ends-with`]($str.ends-with) function on strings
  - Fixed bug with destructuring in combination with break, continue, and return
  - Fixed argument types of [hyperbolic functions]($calc.cosh), they don't allow
    angles anymore (**Breaking change**)
  - Renamed some color methods: `rgba` becomes `to-rgba`, `cmyk` becomes
    `to-cmyk`, and `luma` becomes `to-luma` (**Breaking change**)

- Export
  - Added SVG export
    (thanks to [@Enter-tainer](https://github.com/Enter-tainer))
  - Fixed bugs with PDF font embedding
  - Added support for page labels that reflect the
    [page numbering]($page.numbering) style in the PDF

- Text and Layout
  - Added [`highlight`]($highlight) function for highlighting text with a
    background color
  - Added [`polygon.regular`]($polygon.regular) function for drawing a regular
    polygon
  - Added support for tabs in [`raw`]($raw) elements alongside
    [`tab-width`]($raw.tab-size) parameter
  - The layout engine now tries to prevent "runts" (final lines consisting of
    just a single word)
  - Added Finnish translations
  - Added hyphenation support for Polish
  - Improved handling of consecutive smart quotes of different kinds
  - Fixed vertical alignments for [`number-align`]($page.number-align) argument
    on page function (**Breaking change**)
  - Fixed weak pagebreaks after counter updates
  - Fixed missing text in SVG when the text font is set to "New Computer Modern"
  - Fixed translations for Chinese
  - Fixed crash for empty text in show rule
  - Fixed leading spaces when there's a linebreak after a number and a comma
  - Fixed placement of floating elements in columns and other containers
  - Fixed sizing of block containing just a single box

- Math
  - Added support for [augmented matrices]($math.mat.augment)
  - Removed support for automatic matching of fences like `|` and `||` as
    there were too many false positives. You can use functions like
    [`abs`]($math.abs) or [`norm`]($math.norm) or an explicit [`lr`]($math.lr)
    call instead. (**Breaking change**)
  - Fixed spacing after number with decimal point in math
  - Fixed bug with primes in subscript
  - Fixed weak spacing
  - Fixed crash when text within math contains a newline

- Tooling and Diagnostics
  - Added hints when trying to call a function stored in a dictionary without
    extra parentheses
  - Fixed hint when referencing an equation without numbering
  - Added more details to some diagnostics (e.g. when SVG decoding fails)

- Command line interface
  - Added `typst update` command for self-updating the CLI
    (thanks to [@jimvdl](https://github.com/jimvdl))
  - Added download progress indicator for packages and updates
  - Added `--format` argument to explicitly specify the output format
  - The CLI now respects proxy configuration through environment variables and
    has a new `--cert` option for setting a custom CA certificate
  - Fixed crash when field wasn't present and `--one` is passed to `typst query`

- Miscellaneous Improvements
  - Added [page setup guide]($guides/page-setup-guide)
  - Added [`figure.caption`]($figure.caption) function that can be used for
    simpler figure customization (**Breaking change** because `it.caption` now
    renders the full caption with supplement in figure show rules and manual
    outlines)
  - Moved `caption-pos` argument to `figure.caption` function and renamed it to
    `position` (**Breaking change**)
  - Added [`separator`]($figure.caption.separator) argument to `figure.caption`
    function
  - Added support for combination of and/or and before/after
    [selectors]($selector)
  - Packages can now specify a
    [minimum compiler version](https://github.com/typst/packages#package-format)
    they require to work
  - Fixed parser bug where method calls could be moved onto their own line for
    `[#let]` expressions in markup (**Breaking change**)
  - Fixed bugs in sentence and title case conversion for bibliographies
  - Fixed supplements for alphanumeric and author-title bibliography styles
  - Fixed off-by-one error in APA bibliography style

- Development
  - Made `Span` and `FileId` more type-safe so that all error conditions must be
    handled by `World` implementors

<contributors from="v0.7.0" to="v0.8.0" />

## Version 0.7.0 (August 7, 2023) { #v0.7.0 }
- Text and Layout
  - Added support for floating figures through the
    [`placement`]($figure.placement) argument on the figure function
  - Added support for arbitrary floating content through the
    [`float`]($place.float) argument on the place function
  - Added support for loading `.sublime-syntax` files as highlighting
    [syntaxes]($raw.syntaxes) for raw blocks
  - Added support for loading `.tmTheme` files as highlighting
    [themes]($raw.theme) for raw blocks
  - Added _bounds_ option to [`top-edge`]($text.top-edge) and
    [`bottom-edge`]($text.bottom-edge) arguments of text function for tight
    bounding boxes
  - Removed nonsensical top- and bottom-edge options, e.g. _ascender_ for the
    bottom edge (**Breaking change**)
  - Added [`script`]($text.script) argument to text function
  - Added [`alternative`]($smartquote.alternative) argument to smart quote
    function
  - Added basic i18n for Japanese
  - Added hyphenation support for `nb` and `nn` language codes in addition to
    `no`
  - Fixed positioning of [placed elements]($place) in containers
  - Fixed overflowing containers due to optimized line breaks

- Export
  - Greatly improved export of SVG images to PDF. Many thanks to
    [@LaurenzV](https://github.com/LaurenzV) for their work on this.
  - Added support for the alpha channel of RGBA colors in PDF export
  - Fixed a bug with PPI (pixels per inch) for PNG export

- Math
  - Improved layout of primes (e.g. in `[$a'_1$]`)
  - Improved display of multi-primes (e.g. in `[$a''$]`)
  - Improved layout of [roots]($math.root)
  - Changed relations to show attachments as [limits]($math.limits) by default
    (e.g. in `[$a ->^x b$]`)
  - Large operators and delimiters are now always vertically centered
  - [Boxes]($box) in equations now sit on the baseline instead of being
    vertically centered by default. Notably, this does not affect
    [blocks]($block) because they are not inline elements.
  - Added support for [weak spacing]($h.weak)
  - Added support for OpenType character variants
  - Added support for customizing the [math class]($math.class) of content
  - Fixed spacing around `.`, `\/`, and `...`
  - Fixed spacing between closing delimiters and large operators
  - Fixed a bug with math font weight selection
  - Symbols and Operators (**Breaking changes**)
    - Added `id`, `im`, and `tr` text [operators]($math.op)
    - Renamed `ident` to `equiv` with alias `eq.triple` and removed
      `ident.strict` in favor of `eq.quad`
    - Renamed `ast.sq` to `ast.square` and `integral.sq` to `integral.square`
    - Renamed `.eqq` modifier to `.equiv` (and `.neqq` to `.nequiv`) for
      `tilde`, `gt`, `lt`, `prec`, and `succ`
    - Added `emptyset` as alias for `nothing`
    - Added `lt.curly` and `gt.curly` as aliases for `prec` and `succ`
    - Added `aleph`, `beth`, and `gimmel` as alias for `alef`, `bet`, and
      `gimel`

- Scripting
  - Fields
    - Added `abs` and `em` field to [lengths]($length)
    - Added `ratio` and `length` field to [relative lengths]($relative)
    - Added `x` and `y` field to [2d alignments]($align.alignment)
    - Added `paint`, `thickness`, `cap`, `join`, `dash`, and `miter-limit` field
      to [strokes]($stroke)
  - Accessor and utility methods
    - Added [`dedup`]($array.dedup) method to arrays
    - Added `pt`, `mm`, `cm`, and `inches` method to [lengths]($length)
    - Added `deg` and `rad` method to [angles]($angle)
    - Added `kind`, `hex`, `rgba`, `cmyk`, and `luma` method to [colors]($color)
    - Added `axis`, `start`, `end`, and `inv` method to [directions]($stack.dir)
    - Added `axis` and `inv` method to [alignments]($align.alignment)
    - Added `inv` method to [2d alignments]($align.alignment)
    - Added `start` argument to [`enumerate`]($array.enumerate) method on arrays
  - Added [`color.mix`]($color.mix) function
  - Added `mode` and `scope` arguments to [`eval`]($eval) function
  - Added [`bytes`]($bytes) type for holding large byte buffers
    - Added [`encoding`]($read.encoding) argument to read function to read a
      file as bytes instead of a string
    - Added [`image.decode`]($image.decode) function for decoding an image
      directly from a string or bytes
    - Added [`bytes`]($bytes) function for converting a string or an array of
      integers to bytes
    - Added [`array`]($array) function for converting bytes to an array of
      integers
    - Added support for converting bytes to a string with the [`str`]($str)
      function

- Tooling and Diagnostics
  - Added support for compiler warnings
  - Added warning when compilation does not converge within five attempts due to
    intense use of introspection features
  - Added warnings for empty emphasis (`__` and `**`)
  - Improved error message for invalid field assignments
  - Improved error message after single `#`
  - Improved error message when a keyword is used where an identifier is
    expected
  - Fixed parameter autocompletion for functions that are in modules
  - Import autocompletion now only shows the latest package version until a
    colon is typed
  - Fixed autocompletion for dictionary key containing a space
  - Fixed autocompletion for for loops

- Command line interface
  - Added `typst query` subcommand to execute a
    [query]($reference/meta/query/#command-line-queries) on the command line
  - The `--root` and `--font-paths` arguments cannot appear in front of the
    command anymore (**Breaking change**)
  - Local and cached packages are now stored in directories of the form
    `[{namespace}/{name}/{version}]` instead of `[{namespace}/{name}-{version}]`
    (**Breaking change**)
  - Now prioritizes explicitly given fonts (via `--font-paths`) over system and
    embedded fonts when both exist
  - Fixed `typst watch` not working with some text editors
  - Fixed displayed compilation time (now includes export)

- Miscellaneous Improvements
  - Added [`bookmarked`]($heading.bookmarked) argument to heading to control
    whether a heading becomes part of the PDF outline
  - Added [`caption-pos`]($figure.caption.position) argument to control the
    position of a figure's caption
  - Added [`metadata`]($metadata) function for exposing an arbitrary value to
    the introspection system
  - Fixed that a [`state`]($state) was identified by the pair `(key, init)`
    instead of just its `key`
  - Improved indent logic of [enumerations]($enum). Instead of requiring at
    least as much indent as the end of the marker, they now require only one
    more space indent than the start of the marker. As a result, even long
    markers like `12.` work with just 2 spaces of indent.
  - Fixed bug with indent logic of [`raw`]($raw) blocks
  - Fixed a parsing bug with dictionaries

- Development
  - Extracted parser and syntax tree into `typst-syntax` crate
  - The `World::today` implementation of Typst dependants may need fixing if
    they have the same [bug](https://github.com/typst/typst/issues/1842) that
    the CLI world had

<contributors from="v0.6.0" to="v0.7.0" />

## Version 0.6.0 (June 30, 2023) { #v0.6.0 }
- Package Management
  - Typst now has built-in [package management]($scripting/#packages)
  - You can import [published]($packages) community packages or create and use
    [system-local](https://github.com/typst/packages#local-packages) ones
  - Published packages are also supported in the web app

- Math
  - Added support for optical size variants of glyphs in math mode
  - Added argument to enable [`limits`]($math.limits) conditionally depending on
    whether the equation is set in [`display`]($math.display) or
    [`inline`]($math.inline) style
  - Added `gt.eq.slant` and `lt.eq.slant` symbols
  - Increased precedence of factorials in math mode (`[$1/n!$]` works correctly
    now)
  - Improved [underlines]($math.underline) and [overlines]($math.overline) in
    math mode
  - Fixed usage of [`limits`]($math.limits) function in show rules
  - Fixed bugs with line breaks in equations

- Text and Layout
  - Added support for alternating page [margins]($page.margin) with the `inside`
    and `outside` keys
  - Added support for specifying the page [`binding`]($page.binding)
  - Added [`to`]($pagebreak.to) argument to pagebreak function to skip to the
    next even or odd page
  - Added basic i18n for a few more languages (TR, SQ, TL)
  - Fixed bug with missing table row at page break
  - Fixed bug with [underlines]($underline)
  - Fixed bug superfluous table lines
  - Fixed smart quotes after line breaks
  - Fixed a crash related to text layout

- Command line interface
  - **Breaking change:** Added requirement for `--root`/`TYPST_ROOT` directory
    to contain the input file because it designates the _project_ root. Existing
    setups that use `TYPST_ROOT` to emulate package management should switch to
    [local packages](https://github.com/typst/packages#local-packages)
  - **Breaking change:** Now denies file access outside of the project root
  - Added support for local packages and on-demand package download
  - Now watches all relevant files, within the root and all packages
  - Now displays compilation time

- Miscellaneous Improvements
  - Added [`outline.entry`]($outline.entry) to customize outline entries with
    show rules
  - Added some hints for error messages
  - Added some missing syntaxes for [`raw`]($raw) highlighting
  - Improved rendering of rotated images in PNG export and web app
  - Made [footnotes]($footnote) reusable and referenceable
  - Fixed bug with citations and bibliographies in [`locate`]($locate)
  - Fixed inconsistent tense in documentation

- Development
  - Added [contribution guide](https://github.com/typst/typst/blob/main/CONTRIBUTING.md)
  - Reworked `World` interface to accommodate for package management and make
    it a bit simpler to implement _(Breaking change for implementors)_

<contributors from="v0.5.0" to="v0.6.0" />

## Version 0.5.0 (June 9, 2023) { #v0.5.0 }
- Text and Layout
  - Added [`raw`]($raw) syntax highlighting for many more languages
  - Added support for Korean [numbering]($numbering)
  - Added basic i18n for a few more languages (NL, SV, DA)
  - Improved line breaking for East Asian languages
  - Expanded functionality of outline [`indent`]($outline.indent) property
  - Fixed footnotes in columns
  - Fixed page breaking bugs with [footnotes]($footnote)
  - Fixed bug with handling of footnotes in lists, tables, and figures
  - Fixed a bug with CJK punctuation adjustment
  - Fixed a crash with rounded rectangles
  - Fixed alignment of [`line`]($line) elements

- Math
  - **Breaking change:** The syntax rules for mathematical
    [attachments]($math.attach) were improved: `[$f^abs(3)$]` now parses as
    `[$f^(abs(3))$]` instead of `[$(f^abs)(3)$]`. To disambiguate, add a space:
    `[$f^zeta (3)$]`.
  - Added [forced size]($category/math/sizes) commands for math (e.g.,
    [`display`]($math.display))
  - Added [`supplement`]($math.equation.supplement) parameter to
    [`equation`]($math.equation), used by [references]($ref)
  - New [symbols]($category/symbols/sym): `bullet`, `xor`, `slash.big`,
    `sigma.alt`, `tack.r.not`, `tack.r.short`, `tack.r.double.not`
  - Fixed a bug with symbols in matrices
  - Fixed a crash in the [`attach`]($math.attach) function

- Scripting
  - Added new [`datetime`]($datetime) type and
    [`datetime.today`]($datetime.today) to retrieve the current date
  - Added [`str.from-unicode`]($str.from-unicode) and
    [`str.to-unicode`]($str.to-unicode) functions
  - Added [`fields`]($content.fields) method on content
  - Added `base` parameter to [`str`]($str) function
  - Added [`calc.exp`]($calc.exp) and [`calc.ln`]($calc.ln)
  - Improved accuracy of [`calc.pow`]($calc.pow) and [`calc.log`]($calc.log) for
    specific bases
  - Fixed [removal]($dictionary.remove) order for dictionary
  - Fixed `.at(default: ..)` for [strings]($str.at) and [content]($content.at)
  - Fixed field access on styled elements
  - Removed deprecated `calc.mod` function

- Command line interface
  - Added PNG export via `typst compile source.typ output-{n}.png`. The output
    path must contain `[{n}]` if the document has multiple pages.
  - Added `--diagnostic-format=short` for Unix-style short diagnostics
  - Doesn't emit color codes anymore if stderr isn't a TTY
  - Now sets the correct exit when invoked with a non-existent file
  - Now ignores UTF-8 BOM in Typst files

- Miscellaneous Improvements
  - Improved errors for mismatched delimiters
  - Improved error message for failed length comparisons
  - Fixed a bug with images not showing up in Apple Preview
  - Fixed multiple bugs with the PDF outline
  - Fixed citations and other searchable elements in [`hide`]($hide)
  - Fixed bugs with [reference supplements]($ref.supplement)
  - Fixed Nix flake

<contributors from="v0.4.0" to="v0.5.0" />

## Version 0.4.0 (May 20, 2023) { #v0.4.0 }
- Footnotes
  - Implemented support for footnotes
  - The [`footnote`]($footnote) function inserts a footnote
  - The [`footnote.entry`]($footnote.entry) function can be used to customize
    the footnote listing
  - The `{"chicago-notes"}` [citation style]($cite.style) is now available

- Documentation
  - Added a [Guide for LaTeX users]($guides/guide-for-latex-users)
  - Now shows default values for optional arguments
  - Added richer outlines in "On this Page"
  - Added initial support for search keywords: "Table of Contents" will now find
    the [outline]($outline) function. Suggestions for more keywords are welcome!
  - Fixed issue with search result ranking
  - Fixed many more small issues

- Math
  - **Breaking change**: Alignment points (`&`) in equations now alternate
    between left and right alignment
  - Added support for writing roots with Unicode: For example, `[$root(x+y)$]`
    can now also be written as `[$√(x+y)$]`
  - Fixed uneven vertical [`attachment`]($math.attach) alignment
  - Fixed spacing on decorated elements (e.g., spacing around a
    [canceled]($math.cancel) operator)
  - Fixed styling for stretchable symbols
  - Added `tack.r.double`, `tack.l.double`, `dotless.i` and `dotless.j`
    [symbols]($category/symbols/sym)
  - Fixed show rules on symbols (e.g. `{show sym.tack: set text(blue)}`)
  - Fixed missing rename from `ast.op` to `ast` that should have been in the
    previous release

- Scripting
  - Added function scopes: A function can now hold related definitions in its
    own scope, similar to a module. The new [`assert.eq`]($assert.eq) function,
    for instance, is part of the [`assert`]($assert) function's scope. Note that
    function scopes are currently only available for built-in functions.
  - Added [`assert.eq`]($assert.eq) and [`assert.ne`]($assert.ne) functions for
    simpler equality and inequality assertions with more helpful error messages
  - Exposed [list]($list.item), [enum]($enum.item), and [term list]($terms.item)
    items in their respective functions' scope
  - The `at` methods on [strings]($str.at), [arrays]($array.at),
    [dictionaries]($dictionary.at), and [content]($content.at) now support
    specifying a default value
  - Added support for passing a function to [`replace`]($str.replace) that is
    called with each match.
  - Fixed [replacement]($str.replace) strings: They are now inserted completely
    verbatim instead of supporting the previous (unintended) magic dollar syntax
    for capture groups
  - Fixed bug with trailing placeholders in destructuring patterns
  - Fixed bug with underscore in parameter destructuring
  - Fixed crash with nested patterns and when hovering over an invalid pattern
  - Better error messages when casting to an [integer]($int) or [float]($float)
    fails

- Text and Layout
  - Implemented sophisticated CJK punctuation adjustment
  - Disabled [overhang]($text.overhang) for CJK punctuation
  - Added basic translations for Traditional Chinese
  - Fixed [alignment]($raw.align) of text inside raw blocks (centering a raw
    block, e.g. through a figure, will now keep the text itself left-aligned)
  - Added support for passing a array instead of a function to configure table
    cell [alignment]($table.align) and [fill]($table.fill) per column
  - Fixed automatic figure [`kind`]($figure.kind) detection
  - Made alignment of [enum numbers]($enum.number-align) configurable,
    defaulting to `end`
  - Figures can now be made breakable with a show-set rule for blocks in figure
  - Initial fix for smart quotes in RTL languages

- Export
  - Fixed ligatures in PDF export: They are now copyable and searchable
  - Exported PDFs now embed ICC profiles for images that have them
  - Fixed export of strokes with zero thickness

- Web app
  - Projects can now contain folders
  - Added upload by drag-and-drop into the file panel
  - Files from the file panel can now be dragged into the editor to insert them
    into a Typst file
  - You can now copy-paste images and other files from your computer directly
    into the editor
  - Added a button to resend confirmation email
  - Added an option to invert preview colors in dark mode
  - Added tips to the loading screen and the Help menu. Feel free to propose
    more!
  - Added syntax highlighting for YAML files
  - Allowed middle mouse button click on many buttons to navigate into a new tab
  - Allowed more project names
  - Fixed overridden Vim mode keybindings
  - Fixed many bugs regarding file upload and more

- Miscellaneous Improvements
  - Improved performance of counters, state, and queries
  - Improved incremental parsing for more efficient recompilations
  - Added support for `.yaml` extension in addition to `.yml` for bibliographies
  - The CLI now emits escape codes only if the output is a TTY
  - For users of the `typst` crate: The `Document` is now `Sync` again and the
    `World` doesn't have to be `'static` anymore

<contributors from="v0.3.0" to="v0.4.0" />

## Version 0.3.0 (April 26, 2023) { #v0.3.0 }
- **Breaking changes:**
  - Renamed a few symbols: What was previous `dot.op` is now just `dot` and the
    basic dot is `dot.basic`. The same applies to `ast` and `tilde`.
  - Renamed `mod` to [`rem`]($calc.rem) to more accurately reflect the
    behaviour. It will remain available as `mod` until the next update as a
    grace period.
  - A lone underscore is not a valid identifier anymore, it can now only be used
    in patterns
  - Removed `before` and `after` arguments from [`query`]($query). This is now
    handled through flexible [selectors]($selector) combinator methods
  - Added support for [attachments]($math.attach) (sub-, superscripts) that
    precede the base symbol. The `top` and `bottom` arguments have been renamed
    to `t` and `b`.

- New features
  - Added support for more complex [strokes]($stroke) (configurable caps, joins,
    and dash patterns)
  - Added [`cancel`]($math.cancel) function for equations
  - Added support for [destructuring]($scripting/#bindings) in argument lists
    and assignments
  - Added [`alt`]($image.alt) text argument to image function
  - Added [`toml`]($toml) function for loading data from a TOML file
  - Added [`zip`]($array.zip), [`sum`]($array.sum), and
    [`product`]($array.product) methods for arrays
  - Added `fact`, `perm`, `binom`, `gcd`, `lcm`, `atan2`, `quo`, `trunc`, and
    `fract` [calculation]($category/foundations/calc) functions

- Improvements
  - Text in SVGs now displays properly
  - Typst now generates a PDF heading outline
  - [References]($ref) now provides the referenced element as a field in show
    rules
  - Refined linebreak algorithm for better Chinese justification
  - Locations are now a valid kind of selector
  - Added a few symbols for algebra
  - Added Spanish smart quote support
  - Added [`selector`]($selector) function to turn a selector-like value into a
    selector on which combinator methods can be called
  - Improved some error messages
  - The outline and bibliography headings can now be styled with show-set rules
  - Operations on numbers now produce an error instead of overflowing

- Bug fixes
  - Fixed wrong linebreak before punctuation that follows inline equations,
    citations, and other elements
  - Fixed a bug with [argument sinks]($arguments)
  - Fixed strokes with thickness zero
  - Fixed hiding and show rules in math
  - Fixed alignment in matrices
  - Fixed some alignment bugs in equations
  - Fixed grid cell alignment
  - Fixed alignment of list marker and enum markers in presence of global
    alignment settings
  - Fixed [path]($path) closing
  - Fixed compiler crash with figure references
  - A single trailing line breaks is now ignored in math, just like in text

- Command line interface
  - Font path and compilation root can now be set with the environment variables
    `TYPST_FONT_PATHS` and `TYPST_ROOT`
  - The output of `typst fonts` now includes the embedded fonts

- Development
  - Added instrumentation for debugging and optimization
  - Added `--update` flag and `UPDATE_EXPECT` environment variable to update
    reference images for tests
  - You can now run a specific subtest with `--subtest`
  - Tests now run on multiple threads

<contributors from="v0.2.0" to="v0.3.0" />

## Version 0.2.0 (April 11, 2023) { #v0.2.0 }
- **Breaking changes:**
  - Removed support for iterating over index and value in
    [for loops]($scripting/#loops). This is now handled via unpacking and
    enumerating. Same goes for the [`map`]($array.map) method.
  - [Dictionaries]($dictionary) now iterate in insertion order instead of
    alphabetical order.

- New features
  - Added [unpacking syntax]($scripting/#bindings) for let bindings, which
    allows things like `{let (1, 2) = array}`
  - Added [`enumerate`]($array.enumerate) method
  - Added [`path`]($path) function for drawing Bézier paths
  - Added [`layout`]($layout) function to access the size of the surrounding
    page or container
  - Added `key` parameter to [`sorted`]($array.sorted) method

- Command line interface
  - Fixed `--open` flag blocking the program
  - New Computer Modern font is now embedded into the binary
  - Shell completions and man pages can now be generated by setting the
    `GEN_ARTIFACTS` environment variable to a target directory and then building
    Typst

- Miscellaneous improvements
  - Fixed page numbering in outline
  - Added basic i18n for a few more languages
    (AR, NB, CS, NN, PL, SL, ES, UA, VI)
  - Added a few numbering patterns (Ihora, Chinese)
  - Added `sinc` [operator]($math.op)
  - Fixed bug where math could not be hidden with [`hide`]($hide)
  - Fixed sizing issues with box, block, and shapes
  - Fixed some translations
  - Fixed inversion of "R" in [`cal`]($math.cal) and [`frak`]($math.frak) styles
  - Fixed some styling issues in math
  - Fixed supplements of references to headings
  - Fixed syntax highlighting of identifiers in certain scenarios
  - [Ratios]($ratio) can now be multiplied with more types and be converted to
    [floats]($float) with the [`float`]($float) function

<contributors from="v0.1.0" to="v0.2.0" />

## Version 0.1.0 (April 04, 2023) { #v0.1.0 }
- **Breaking changes:**
  - When using the CLI, you now have to use subcommands:
    - `typst compile file.typ` or `typst c file.typ` to create a PDF
    - `typst watch file.typ` or `typst w file.typ` to compile and watch
    - `typst fonts` to list all fonts
  - Manual counters now start at zero. Read the "How to step" section
    [here]($counter) for more details
  - The [bibliography styles]($bibliography.style) `{"author-date"}` and
    `{"author-title"}` were renamed to `{"chicago-author-date"}` and
    `{"chicago-author-title"}`

- Figure improvements
  - Figures now automatically detect their content and adapt their behaviour.
    Figures containing tables, for instance, are automatically prefixed with
    "Table X" and have a separate counter
  - The figure's supplement (e.g. "Figure" or "Table") can now be customized
  - In addition, figures can now be completely customized because the show rule
    gives access to the automatically resolved kind, supplement, and counter

- Bibliography improvements
  - The [`bibliography`]($bibliography) now also accepts multiple bibliography
    paths (as an array)
  - Parsing of BibLaTeX files is now more permissive (accepts non-numeric
    edition, pages, volumes, dates, and Jabref-style comments; fixed
    abbreviation parsing)
  - Labels and references can now include `:` and `.` except at the end
  - Fixed APA bibliography ordering

- Drawing additions
  - Added [`polygon`]($polygon) function for drawing polygons
  - Added support for clipping in [boxes]($box.clip) and [blocks]($block.clip)

- Command line interface
  - Now returns with non-zero status code if there is an error
  - Now watches the root directory instead of the current one
  - Now puts the PDF file next to input file by default
  - Now accepts more kinds of input files (e.g. `/dev/stdin`)
  - Added `--open` flag to directly open the PDF

- Miscellaneous improvements
  - Added [`yaml`]($yaml) function to load data from YAML files
  - Added basic i18n for a few more languages (IT, RU, ZH, FR, PT)
  - Added numbering support for Hebrew
  - Added support for [integers]($int) with base 2, 8, and 16
  - Added symbols for double bracket and laplace operator
  - The [`link`]($link) function now accepts [labels]($label)
  - The link syntax now allows more characters
  - Improved justification of Japanese and Chinese text
  - Calculation functions behave more consistently w.r.t to non-real results
  - Replaced deprecated angle brackets
  - Reduced maximum function call depth from 256 to 64
  - Fixed [`first-line-indent`]($par.first-line-indent) being not applied when a
    paragraph starts with styled text
  - Fixed extraneous spacing in unary operators in equations
  - Fixed block spacing, e.g. in `{block(above: 1cm, below: 1cm, ..)}`
  - Fixed styling of text operators in math
  - Fixed invalid parsing of language tag in raw block with a single backtick
  - Fixed bugs with displaying counters and state
  - Fixed crash related to page counter
  - Fixed crash when [`symbol`]($symbol) function was called without arguments
  - Fixed crash in bibliography generation
  - Fixed access to label of certain content elements
  - Fixed line number in error message for CSV parsing
  - Fixed invalid autocompletion after certain markup elements

<contributors from="v23-03-28" to="v0.1.0" />

## March 28, 2023 { #_ }
- **Breaking changes:**
  - Enumerations now require a space after their marker, that is, `[1.ok]` must
    now be written as `[1. ok]`
  - Changed default style for [term lists]($terms): Does not include a colon
    anymore and has a bit more indent

- Command line interface
  - Added `--font-path` argument for CLI
  - Embedded default fonts in CLI binary
  - Fixed build of CLI if `git` is not installed

- Miscellaneous improvements
  - Added support for disabling [matrix]($math.mat) and [vector]($math.vec)
    delimiters. Generally with `[#set math.mat(delim: none)]` or one-off with
    `[$mat(delim: #none, 1, 2; 3, 4)$]`.
  - Added [`separator`]($terms.separator) argument to term lists
  - Added [`round`]($math.round) function for equations
  - Numberings now allow zeros. To reset a counter, you can write
    `[#counter(..).update(0)]`
  - Added documentation for `{page()}` and `{position()}` methods on
    [`location`]($location) type
  - Added symbols for double, triple, and quadruple dot accent
  - Added smart quotes for Norwegian Bokmål
  - Added Nix flake
  - Fixed bibliography ordering in IEEE style
  - Fixed parsing of decimals in math: `[$1.2/3.4$]`
  - Fixed parsing of unbalanced delimiters in fractions: `[$1/(2 (x)$]`
  - Fixed unexpected parsing of numbers as enumerations, e.g. in `[1.2]`
  - Fixed combination of page fill and header
  - Fixed compiler crash if [`repeat`]($repeat) is used in page with automatic
    width
  - Fixed [matrices]($math.mat) with explicit delimiter
  - Fixed [`indent`]($terms.indent) property of term lists
  - Numerous documentation fixes
  - Links in bibliographies are now affected by link styling
  - Fixed hovering over comments in web app

<contributors from="v23-03-21" to="v23-03-28" />

## March 21, 2023 { #_ }
- Reference and bibliography management
  - [Bibliographies]($bibliography) and [citations]($cite) (currently supported
    styles are APA, Chicago Author Date, IEEE, and MLA)
  - You can now [reference]($ref) sections, figures, formulas, and works from
    the bibliography with `[@label]`
  - You can make an element referenceable with a label:
    - `[= Introduction <intro>]`
    - `[$ A = pi r^2 $ <area>]`

- Introspection system for interactions between different parts of the document
  - [`counter`]($counter) function
    - Access and modify counters for pages, headings, figures, and equations
    - Define and use your own custom counters
    - Time travel: Find out what the counter value was or will be at some other
      point in the document (e.g. when you're building a list of figures, you
      can determine the value of the figure counter at any given figure).
    - Counters count in layout order and not in code order
  - [`state`]($state) function
    - Manage arbitrary state across your document
    - Time travel: Find out the value of your state at any position in the
      document
    - State is modified in layout order and not in code order
  - [`query`]($query) function
    - Find all occurrences of an element or a label, either in the whole
      document or before/after some location
    - Link to elements, find out their position on the pages and access their
      fields
    - Example use cases: Custom list of figures or page header with current
      chapter title
  - [`locate`]($locate) function
    - Determines the location of itself in the final layout
    - Can be accessed to get the `page` and `x`, `y` coordinates
    - Can be used with counters and state to find out their values at that
      location
    - Can be used with queries to find elements before or after its location

- New [`measure`]($measure) function
  - Measure the layouted size of elements
  - To be used in combination with the new [`style`]($style) function that lets
    you generate different content based on the style context something is
    inserted into (because that affects the measured size of content)

- Exposed content representation
  - Content is not opaque anymore
  - Content can be compared for equality
  - The tree of content elements can be traversed with code
  - Can be observed in hover tooltips or with [`repr`]($repr)
  - New [methods]($content) on content: `func`, `has`, `at`, and `location`
  - All optional fields on elements are now settable
  - More uniform field names (`heading.title` becomes `heading.body`,
    `list.items` becomes `list.children`, and a few more changes)

- Further improvements
  - Added [`figure`]($figure) function
  - Added [`numbering`]($math.equation.numbering) parameter on equation function
  - Added [`numbering`]($page.numbering) and
    [`number-align`]($page.number-align) parameters on page function
  - The page function's [`header`]($page.header) and [`footer`]($page.footer)
    parameters do not take functions anymore. If you want to customize them
    based on the page number, use the new [`numbering`]($page.numbering)
    parameter or [`counter`]($counter) function instead.
  - Added [`footer-descent`]($page.footer-descent) and
    [`header-ascent`]($page.header-ascent) parameters
  - Better default alignment in header and footer
  - Fixed Arabic vowel placement
  - Fixed PDF font embedding issues
  - Renamed `math.formula` to [`math.equation`]($math.equation)
  - Font family must be a named argument now: `[#set text(font: "..")]`
  - Added support for [hanging indent]($par.hanging-indent)
  - Renamed paragraph `indent` to [`first-line-indent`]($par.first-line-indent)
  - More accurate [logarithm]($calc.log) when base is `2` or `10`
  - Improved some error messages
  - Fixed layout of [`terms`]($terms) list

- Web app improvements
  - Added template gallery
  - Added buttons to insert headings, equations, raw blocks, and references
  - Jump to the source of something by clicking on it in the preview panel
    (works for text, equations, images, and more)
  - You can now upload your own fonts and use them in your project
  - Hover debugging and autocompletion now takes multiple files into account and
    works in show rules
  - Hover tooltips now automatically collapse multiple consecutive equal values
  - The preview now automatically scrolls to the right place when you type
  - Links are now clickable in the preview area
  - Toolbar, preview, and editor can now all be hidden
  - Added autocompletion for raw block language tags
  - Added autocompletion in SVG files
  - New back button instead of four-dots button
  - Lots of bug fixes

## February 25, 2023 { #_ }
- Font changes
  - New default font: Linux Libertine
  - New default font for raw blocks: DejaVu Sans Mono
  - New default font for math: Book weight of New Computer Modern Math
  - Lots of new math fonts available
  - Removed Latin Modern fonts in favor of New Computer Modern family
  - Removed unnecessary smallcaps fonts which are already accessible through the
    corresponding main font and the [`smallcaps`]($smallcaps) function
- Improved default spacing for headings
- Added [`panic`]($panic) function
- Added [`clusters`]($str.clusters) and [`codepoints`]($str.codepoints) methods
  for strings
- Support for multiple authors in [`set document`]($document.author)
- Fixed crash when string is accessed at a position that is not a char boundary
- Fixed semicolon parsing in `[#var ;]`
- Fixed incremental parsing when inserting backslash at end of `[#"abc"]`
- Fixed names of a few font families (including Noto Sans Symbols and New
  Computer Modern families)
- Fixed autocompletion for font families
- Improved incremental compilation for user-defined functions

## February 15, 2023 { #_ }
- [Box]($box) and [block]($block) have gained `fill`, `stroke`, `radius`, and
  `inset` properties
- Blocks may now be explicitly sized, fixed-height blocks can still break across
  pages
- Blocks can now be configured to be [`breakable`]($block.breakable) or not
- [Numbering style]($enum.numbering) can now be configured for nested enums
- [Markers]($list.marker) can now be configured for nested lists
- The [`eval`]($eval) function now expects code instead of markup and returns an
  arbitrary value. Markup can still be evaluated by surrounding the string with
  brackets.
- PDFs generated by Typst now contain XMP metadata
- Link boxes are now disabled in PDF output
- Tables don't produce small empty cells before a pagebreak anymore
- Fixed raw block highlighting bug

## February 12, 2023 { #_ }
- Shapes, images, and transformations (move/rotate/scale/repeat) are now
  block-level. To integrate them into a paragraph, use a [`box`]($box) as with
  other elements.
- A colon is now required in an "everything" show rule: Write `{show: it => ..}`
  instead of `{show it => ..}`. This prevents intermediate states that ruin your
  whole document.
- Non-math content like a shape or table in a math formula is now centered
  vertically
- Support for widow and orphan prevention within containers
- Support for [RTL]($text.dir) in lists, grids, and tables
- Support for explicit `{auto}` sizing for boxes and shapes
- Support for fractional (i.e. `{1fr}`) widths for boxes
- Fixed bug where columns jump to next page
- Fixed bug where list items have no leading
- Fixed relative sizing in lists, squares and grid auto columns
- Fixed relative displacement in [`place`]($place) function
- Fixed that lines don't have a size
- Fixed bug where `{set document(..)}` complains about being after content
- Fixed parsing of `{not in}` operation
- Fixed hover tooltips in math
- Fixed bug where a heading show rule may not contain a pagebreak when an
  outline is present
- Added [`baseline`]($box.baseline) property on [`box`]($box)
- Added [`tg`]($math.op) and [`ctg`]($math.op) operators in math
- Added delimiter setting for [`cases`]($math.cases) function
- Parentheses are now included when accepting a function autocompletion

## February 2, 2023 { #_ }
- Merged text and math symbols, renamed a few symbols (including `infty` to
  `infinity` with the alias `oo`)
- Fixed missing italic mappings
- Math italics correction is now applied properly
- Parentheses now scale in `[$zeta(x/2)$]`
- Fixed placement of large root index
- Fixed spacing in `[$abs(-x)$]`
- Fixed inconsistency between text and identifiers in math
- Accents are now ignored when positioning superscripts
- Fixed vertical alignment in matrices
- Fixed `text` set rule in `raw` show rule
- Heading and list markers now parse consistently
- Allow arbitrary math directly in content

## January 30, 2023 { #_ }
[Go to the announcement blog post.](https://typst.app/blog/2023/january-update)
- New expression syntax in markup/math
  - Blocks cannot be directly embedded in markup anymore
  - Like other expressions, they now require a leading hash
  - More expressions available with hash, including literals (`[#"string"]`)
    as well as field access and method call without space: `[#emoji.face]`
- New import syntax
  - `[#import "module.typ"]` creates binding named `module`
  - `[#import "module.typ": a, b]` or `[#import "module.typ": *]` to import
    items
  - `[#import emoji: face, turtle]` to import from already bound module
- New symbol handling
  - Removed symbol notation
  - Symbols are now in modules: `{sym}`, `{emoji}`, and `{math}`
  - Math module also reexports all of `{sym}`
  - Modified through field access, still order-independent
  - Unknown modifiers are not allowed anymore
  - Support for custom symbol definitions with `symbol` function
  - Symbols now listed in documentation
- New `{math}` module
  - Contains all math-related functions
  - Variables and function calls directly in math (without hash) access this
    module instead of the global scope, but can also access local variables
  - Can be explicitly used in code, e.g. `[#set math.vec(delim: "[")]`
- Delimiter matching in math
   - Any opening delimiters matches any closing one
   - When matched, they automatically scale
   - To prevent scaling, escape them
   - To forcibly match two delimiters, use `lr` function
   - Line breaks may occur between matched delimiters
   - Delimiters may also be unbalanced
   - You can also use the `lr` function to scale the brackets (or just one
     bracket) to a specific size manually
- Multi-line math with alignment
  - The `\` character inserts a line break
  - The `&` character defines an alignment point
  - Alignment points also work for underbraces, vectors, cases, and matrices
  - Multiple alignment points are supported
- More capable math function calls
  - Function calls directly in math can now take code expressions with hash
  - They can now also take named arguments
  - Within math function calls, semicolons turn preceding arguments to arrays to
    support matrices: `[$mat(1, 2; 3, 4)$]`
- Arbitrary content in math
  - Text, images, and other arbitrary content can now be embedded in math
  - Math now also supports font fallback to support e.g. CJK and emoji
- More math features
  - New text operators: `op` function, `lim`, `max`, etc.
  - New matrix function: `mat`
  - New n-ary roots with `root` function: `[$root(3, x)$]`
  - New under- and overbraces, -brackets, and -lines
  - New `abs` and `norm` functions
  - New shorthands: `[|`, `|]`, and `||`
  - New `attach` function, overridable attachments with `script` and `limit`
  - Manual spacing in math, with `h`, `thin`, `med`, `thick` and `quad`
  - Symbols and other content may now be used like a function, e.g.
    `[$zeta(x)$]`
  - Added Fira Math font, removed Noto Sans Math font
  - Support for alternative math fonts through `[#show math.formula: set
    text("Fira Math")]`
- More library improvements
  - New `calc` module, `abs`, `min`, `max`, `even`, `odd` and `mod` moved there
  - New `message` argument on `{assert}` function
  - The `pairs` method on dictionaries now returns an array of length-2 arrays
    instead of taking a closure
  - The method call `{dict.at("key")}` now always fails if `"key"` doesn't exist
    Previously, it was allowed in assignments. Alternatives are `{dict.key = x}`
    and `{dict.insert("key", x)}`.
- Smarter editor functionality
  - Autocompletion for local variables
  - Autocompletion for methods available on a value
  - Autocompletion for symbols and modules
  - Autocompletion for imports
  - Hover over an identifier to see its value(s)
- Further editor improvements
  - New Font menu with previews
  - Single projects may now be shared with share links
  - New dashboard experience if projects are shared with you
  - Keyboard Shortcuts are now listed in the menus and there are more of them
  - New Offline indicator
  - Tooltips for all buttons
  - Improved account protection
  - Moved Status indicator into the error list button
- Further fixes
  - Multiple bug fixes for incremental parser
  - Fixed closure parameter capturing
  - Fixed tons of math bugs
  - Bugfixes for performance, file management, editing reliability
  - Added redirection to the page originally navigated to after signin
