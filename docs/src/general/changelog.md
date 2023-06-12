---
description: |
  Learn what has changed in the latest Typst releases and move your documents
  forward.
---

# Changelog
## Version 0.5.0 (June 9, 2023) { #v0.5.0 }
- Text and Layout
  - Added [`raw`]($func/raw) syntax highlighting for many more languages
  - Added support for Korean [numbering]($func/numbering)
  - Added basic i18n for a few more languages (NL, SV, DA)
  - Improved linebreaking for East Asian languages
  - Expanded functionality of outline [`indent`]($func/outline.indent) property
  - Fixed footnotes in columns
  - Fixed page breaking bugs with [footnotes]($func/footnote)
  - Fixed bug with handling of footnotes in lists, tables, and figures
  - Fixed a bug with CJK punctuation adjustment
  - Fixed a crash with rounded rectangles
  - Fixed alignment of [`line`]($func/line) elements

- Math
  - **Breaking change:** The syntax rules for mathematical
    [attachments]($func/math.attach) were improved: `[$f^abs(3)$]` now parses as
    `[$f^(abs(3))$]` instead of `[$(f^abs)(3)$]`. To disambiguate, add a space:
    `[$f^zeta (3)$]`.
  - Added [forced size]($category/math/sizes) commands for math
    (e.g., [`display`]($func/math.display))
  - Added [`supplement`]($func/math.equation.supplement) parameter to
    [`equation`]($func/math.equation), used by [references]($func/ref)
  - New [symbols]($category/symbols/sym): `bullet`, `xor`, `slash.big`,
    `sigma.alt`, `tack.r.not`, `tack.r.short`, `tack.r.double.not`
  - Fixed a bug with symbols in matrices
  - Fixed a crash in the [`attach`]($func/math.attach) function

- Scripting
  - Added new [`datetime`]($type/datetime) type and
    [`datetime.today()`]($func/datetime.today) to retrieve the current date
  - Added [`str.from-unicode`]($func/str.from-unicode) and
    [`str.to-unicode`]($func/str.to-unicode) functions
  - Added [`fields`]($type/content.fields) method on content
  - Added `base` parameter to [`str`]($func/str) function
  - Added [`calc.exp`]($func/calc.exp) and [`calc.ln`]($func/calc.ln)
  - Improved accuracy of [`calc.pow`]($func/calc.pow) and
    [`calc.log`]($func/calc.log) for specific bases
  - Fixed [removal]($type/dictionary.remove) order for dictionary
  - Fixed `.at(default: ..)` for [strings]($type/string.at) and
    [content]($type/content.at)
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
  - Fixed citations and other searchable elements in [`hide`]($func/hide)
  - Fixed bugs with [reference supplements]($func/ref.supplement)
  - Fixed Nix flake

<contributors from="v0.4.0" to="v0.5.0" />

## Version 0.4.0 (May 20, 2023) { #v0.4.0 }
- Footnotes
  - Implemented support for footnotes
  - The [`footnote`]($func/footnote) function inserts a footnote
  - The [`footnote.entry`]($func/footnote.entry) function can be used to
    customize the footnote listing
  - The `{"chicago-notes"}` [citation style]($func/cite.style) is now available

- Documentation
  - Added a [Guide for LaTeX users]($guides/guide-for-latex-users)
  - Now shows default values for optional arguments
  - Added richer outlines in "On this Page"
  - Added initial support for search keywords: "Table of Contents" will now find
    the [outline]($func/outline) function. Suggestions for more keywords are
    welcome!
  - Fixed issue with search result ranking
  - Fixed many more small issues

- Math
  - **Breaking change**: Alignment points (`&`) in equations now alternate
    between left and right alignment
  - Added support for writing roots with Unicode:
    For example, `[$root(x+y)$]` can now also be written as `[$√(x+y)$]`
  - Fixed uneven vertical [`attachment`]($func/math.attach) alignment
  - Fixed spacing on decorated elements
    (e.g., spacing around a [canceled]($func/math.cancel) operator)
  - Fixed styling for stretchable symbols
  - Added `tack.r.double`, `tack.l.double`, `dotless.i` and `dotless.j`
    [symbols]($category/symbols/sym)
  - Fixed show rules on symbols (e.g. `{show sym.tack: set text(blue)}`)
  - Fixed missing rename from `ast.op` to `ast` that should have been in the
    previous release

- Scripting
  - Added function scopes: A function can now hold related definitions in its
    own scope, similar to a module. The new [`assert.eq`]($func/assert.eq)
    function, for instance, is part of the [`assert`]($func/assert) function's
    scope. Note that function scopes are currently only available for built-in
    functions.
  - Added [`assert.eq`]($func/assert.eq) and [`assert.ne`]($func/assert.ne)
    functions for simpler equality and inequality assertions with more helpful
    error messages
  - Exposed [list]($func/list.item), [enum]($func/enum.item), and
    [term list]($func/terms.item) items in their respective functions' scope
  - The `at` methods on [strings]($type/string.at), [arrays]($type/array.at),
    [dictionaries]($type/dictionary.at), and [content]($type/content.at) now support
    specifying a default value
  - Added support for passing a function to [`replace`]($type/string.replace)
    that is called with each match.
  - Fixed [replacement]($type/string.replace) strings: They are now inserted
    completely verbatim instead of supporting the previous (unintended) magic
    dollar syntax for capture groups
  - Fixed bug with trailing placeholders in destructuring patterns
  - Fixed bug with underscore in parameter destructuring
  - Fixed crash with nested patterns and when hovering over an invalid pattern
  - Better error messages when casting to an [integer]($func/int) or
    [float]($func/float) fails

- Text and Layout
  - Implemented sophisticated CJK punctuation adjustment
  - Disabled [overhang]($func/text.overhang) for CJK punctuation
  - Added basic translations for Traditional Chinese
  - Fixed [alignment]($func/raw.align) of text inside raw blocks (centering a
    raw block, e.g. through a figure, will now keep the text itself
    left-aligned)
  - Added support for passing a array instead of a function to configure table
    cell [alignment]($func/table.align) and [fill]($func/table.fill) per column
  - Fixed automatic figure [`kind`]($func/figure.kind) detection
  - Made alignment of [enum numbers]($func/enum.number-align) configurable,
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
  - For users of the `typst` crate: The `Document` is now `Sync` again and
    the `World` doesn't have to be `'static` anymore

<contributors from="v0.3.0" to="v0.4.0" />

## Version 0.3.0 (April 26, 2023) { #v0.3.0 }
- **Breaking changes:**
  - Renamed a few symbols: What was previous `dot.op` is now just `dot` and the
    basic dot is `dot.basic`. The same applies to `ast` and `tilde`.
  - Renamed `mod` to [`rem`]($func/calc.rem) to more accurately reflect
    the behaviour. It will remain available as `mod` until the next update as a
    grace period.
  - A lone underscore is not a valid identifier anymore, it can now only be used
    in patterns
  - Removed `before` and `after` arguments from [`query`]($func/query). This is
    now handled through flexible [selectors]($type/selector) combinator methods
  - Added support for [attachments]($func/math.attach) (sub-, superscripts) that
    precede the base symbol. The `top` and `bottom` arguments have been renamed
    to `t` and `b`.

- New features
  - Added support for more complex [strokes]($func/line.stroke)
    (configurable caps, joins, and dash patterns)
  - Added [`cancel`]($func/math.cancel) function for equations
  - Added support for [destructuring]($scripting/#bindings) in argument lists
    and assignments
  - Added [`alt`]($func/image.alt) text argument to image function
  - Added [`toml`]($func/toml) function for loading data from a TOML file
  - Added [`zip`]($type/array.zip), [`sum`]($type/array.sum), and
    [`product`]($type/array.product) methods for arrays
  - Added `fact`, `perm`, `binom`, `gcd`, `lcm`, `atan2`, `quo`, `trunc`, and
    `fract` [calculation]($category/calculate)

- Improvements
  - Text in SVGs now displays properly
  - Typst now generates a PDF heading outline
  - [References]($func/ref) now provides the referenced element as a field in
    show rules
  - Refined linebreak algorithm for better Chinese justification
  - Locations are now a valid kind of selector
  - Added a few symbols for algebra
  - Added Spanish smart quote support
  - Added [`selector`]($func/selector) function to turn a selector-like value
    into a selector on which combinator methods can be called
  - Improved some error messages
  - The outline and bibliography headings can now be styled with show-set rules
  - Operations on numbers now produce an error instead of overflowing

- Bug fixes
  - Fixed wrong linebreak before punctuation that follows inline equations,
    citations, and other elements
  - Fixed a bug with [argument sinks]($type/arguments)
  - Fixed strokes with thickness zero
  - Fixed hiding and show rules in math
  - Fixed alignment in matrices
  - Fixed some alignment bugs in equations
  - Fixed grid cell alignment
  - Fixed alignment of list marker and enum markers in presence of global
    alignment settings
  - Fixed [path]($func/path) closing
  - Fixed compiler crash with figure references
  - A single trailing line breaks is now ignored in math, just like in text

- Command line interface
  - Font path and compilation root can now be set with the environment
    variables `TYPST_FONT_PATHS` and `TYPST_ROOT`
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
    enumerating. Same goes for the [`map()`]($type/array.map) method.
  - [Dictionaries]($type/dictionary) now iterate in insertion order instead of
    alphabetical order.

- New features
  - Added [unpacking syntax]($scripting/#bindings) for let bindings, which
    allows things like `{let (1, 2) = array}`
  - Added [`enumerate()`]($type/array.enumerate) method
  - Added [`path`]($func/path) function for drawing Bézier paths
  - Added [`layout`]($func/layout) function to access the size of the
    surrounding page or container
  - Added `key` parameter to [`sorted()`]($type/array.sorted) method

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
  - Added `sinc` [operator]($func/math.op)
  - Fixed bug where math could not be hidden with [`hide`]($func/hide)
  - Fixed sizing issues with box, block, and shapes
  - Fixed some translations
  - Fixed inversion of "R" in [`cal`]($func/math.cal) and
    [`frak`]($func/math.frak) styles
  - Fixed some styling issues in math
  - Fixed supplements of references to headings
  - Fixed syntax highlighting of identifiers in certain scenarios
  - [Ratios]($type/ratio) can now be multiplied with more types and be converted
    to [floats]($type/float) with the [`float`]($func/float) function

<contributors from="v0.1.0" to="v0.2.0" />

## Version 0.1.0 (April 04, 2023) { #v0.1.0 }
- **Breaking changes:**
  - When using the CLI, you now have to use subcommands:
    - `typst compile file.typ` or `typst c file.typ` to create a PDF
    - `typst watch file.typ` or `typst w file.typ` to compile and watch
    - `typst fonts` to list all fonts
  - Manual counters now start at zero. Read the "How to step" section
    [here]($func/counter) for more details
  - The [bibliography styles]($func/bibliography.style)
    `{"author-date"}` and `{"author-title"}` were renamed to
    `{"chicago-author-date"}` and `{"chicago-author-title"}`

- Figure improvements
  - Figures now automatically detect their content and adapt their
    behaviour. Figures containing tables, for instance, are automatically
    prefixed with "Table X" and have a separate counter
  - The figure's supplement (e.g. "Figure" or "Table") can now be customized
  - In addition, figures can now be completely customized because the show rule
    gives access to the automatically resolved kind, supplement, and counter

- Bibliography improvements
  - The [`bibliography`]($func/bibliography) now also accepts multiple
    bibliography paths (as an array)
  - Parsing of BibLaTeX files is now more permissive (accepts non-numeric
    edition, pages, volumes, dates, and Jabref-style comments; fixed
    abbreviation parsing)
  - Labels and references can now include `:` and `.` except at the end
  - Fixed APA bibliography ordering

- Drawing additions
  - Added [`polygon`]($func/polygon) function for drawing polygons
  - Added support for clipping in [boxes]($func/box.clip) and
    [blocks]($func/block.clip)

- Command line interface
  - Now returns with non-zero status code if there is an error
  - Now watches the root directory instead of the current one
  - Now puts the PDF file next to input file by default
  - Now accepts more kinds of input files (e.g. `/dev/stdin`)
  - Added `--open` flag to directly open the PDF

- Miscellaneous improvements
  - Added [`yaml`]($func/yaml) function to load data from YAML files
  - Added basic i18n for a few more languages (IT, RU, ZH, FR, PT)
  - Added numbering support for Hebrew
  - Added support for [integers]($type/integer) with base 2, 8, and 16
  - Added symbols for double bracket and laplace operator
  - The [`link`]($func/link) function now accepts [labels]($func/label)
  - The link syntax now allows more characters
  - Improved justification of Japanese and Chinese text
  - Calculation functions behave more consistently w.r.t to non-real results
  - Replaced deprecated angle brackets
  - Reduced maximum function call depth from 256 to 64
  - Fixed [`first-line-indent`]($func/par.first-line-indent) being not applied
    when a paragraph starts with styled text
  - Fixed extraneous spacing in unary operators in equations
  - Fixed block spacing, e.g. in `{block(above: 1cm, below: 1cm, ..)}`
  - Fixed styling of text operators in math
  - Fixed invalid parsing of language tag in raw block with a single backtick
  - Fixed bugs with displaying counters and state
  - Fixed crash related to page counter
  - Fixed crash when [`symbol`]($func/symbol) function was called without
    arguments
  - Fixed crash in bibliography generation
  - Fixed access to label of certain content elements
  - Fixed line number in error message for CSV parsing
  - Fixed invalid autocompletion after certain markup elements

<contributors from="v23-03-28" to="v0.1.0" />

## March 28, 2023
- **Breaking changes:**
  - Enumerations now require a space after their marker, that is, `[1.ok]` must
    now be written as `[1. ok]`
  - Changed default style for [term lists]($func/terms): Does not include a
    colon anymore and has a bit more indent

- Command line interface
  - Added `--font-path` argument for CLI
  - Embedded default fonts in CLI binary
  - Fixed build of CLI if `git` is not installed

- Miscellaneous improvements
  - Added support for disabling [matrix]($func/math.mat) and
    [vector]($func/math.vec) delimiters. Generally with
    `[#set math.mat(delim: none)]` or one-off with
    `[$mat(delim: #none, 1, 2; 3, 4)$]`.
  - Added [`separator`]($func/terms.separator) argument to term lists
  - Added [`round`]($func/math.round) function for equations
  - Numberings now allow zeros. To reset a counter, you can write
    `[#counter(..).update(0)]`
  - Added documentation for `{page()}` and `{position()}` methods on
    [`location`]($func/locate) type
  - Added symbols for double, triple, and quadruple dot accent
  - Added smart quotes for Norwegian Bokmål
  - Added Nix flake
  - Fixed bibliography ordering in IEEE style
  - Fixed parsing of decimals in math: `[$1.2/3.4$]`
  - Fixed parsing of unbalanced delimiters in fractions: `[$1/(2 (x)$]`
  - Fixed unexpected parsing of numbers as enumerations, e.g. in `[1.2]`
  - Fixed combination of page fill and header
  - Fixed compiler crash if [`repeat`]($func/repeat) is used in page with
    automatic width
  - Fixed [matrices]($func/math.mat) with explicit delimiter
  - Fixed [`indent`]($func/terms.indent) property of term lists
  - Numerous documentation fixes
  - Links in bibliographies are now affected by link styling
  - Fixed hovering over comments in web app

<contributors from="v23-03-21" to="v23-03-28" />

## March 21, 2023
- Reference and bibliography management
  - [Bibliographies]($func/bibliography) and [citations]($func/cite) (currently
    supported styles are APA, Chicago Author Date, IEEE, and MLA)
  - You can now [reference]($func/ref) sections, figures, formulas, and works
    from the bibliography with `[@label]`
  - You can make an element referenceable with a label:
    - `[= Introduction <intro>]`
    - `[$ A = pi r^2 $ <area>]`

- Introspection system for interactions between different parts of the document
  - [`counter`]($func/counter) function
    - Access and modify counters for pages, headings, figures, and equations
    - Define and use your own custom counters
    - Time travel: Find out what the counter value was or will be at some other
      point in the document (e.g. when you're building a list of figures, you
      can determine the value of the figure counter at any given figure).
    - Counters count in layout order and not in code order
  - [`state`]($func/state) function
    - Manage arbitrary state across your document
    - Time travel: Find out the value of your state at any position in the
      document
    - State is modified in layout order and not in code order
  - [`query`]($func/query) function
    - Find all occurrences of an element or a label, either in the whole document
      or before/after some location
    - Link to elements, find out their position on the pages and access their
      fields
    - Example use cases: Custom list of figures or page header with current
      chapter title
  - [`locate`]($func/locate) function
    - Determines the location of itself in the final layout
    - Can be accessed to get the `page` and `x`, `y` coordinates
    - Can be used with counters and state to find out their values at that
      location
    - Can be used with queries to find elements before or after its location

- New [`measure`]($func/measure) function
  - Measure the layouted size of elements
  - To be used in combination with the new [`style`]($func/style) function that
    lets you generate different content based on the style context something is
    inserted into (because that affects the measured size of content)

- Exposed content representation
  - Content is not opaque anymore
  - Content can be compared for equality
  - The tree of content elements can be traversed with code
  - Can be observed in hover tooltips or with [`repr`]($func/repr)
  - New [methods]($type/content) on content: `func`, `has`, `at`, and `location`
  - All optional fields on elements are now settable
  - More uniform field names (`heading.title` becomes `heading.body`,
    `list.items` becomes `list.children`, and a few more changes)

- Further improvements
  - Added [`figure`]($func/figure) function
  - Added [`numbering`]($func/math.equation.numbering) parameter on equation function
  - Added [`numbering`]($func/page.numbering) and
    [`number-align`]($func/page.number-align) parameters on page function
  - The page function's [`header`]($func/page.header) and
    [`footer`]($func/page.footer) parameters do not take functions anymore. If
    you want to customize them based on the page number, use the new
    [`numbering`]($func/page.numbering) parameter or [`counter`]($func/counter)
    function instead.
  - Added [`footer-descent`]($func/page.footer-descent) and
    [`header-ascent`]($func/page.header-ascent) parameters
  - Better default alignment in header and footer
  - Fixed Arabic vowel placement
  - Fixed PDF font embedding issues
  - Renamed `math.formula` to [`math.equation`]($func/math.equation)
  - Font family must be a named argument now: `[#set text(font: "..")]`
  - Added support for [hanging indent]($func/par.hanging-indent)
  - Renamed paragraph `indent` to
    [`first-line-indent`]($func/par.first-line-indent)
  - More accurate [logarithm]($func/calc.log) when base is `2` or `10`
  - Improved some error messages
  - Fixed layout of [`terms`]($func/terms) list

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

## February 25, 2023
- Font changes
  - New default font: Linux Libertine
  - New default font for raw blocks: DejaVu Sans Mono
  - New default font for math: Book weight of New Computer Modern Math
  - Lots of new math fonts available
  - Removed Latin Modern fonts in favor of New Computer Modern family
  - Removed unnecessary smallcaps fonts which are already accessible through
    the corresponding main font and the [`smallcaps`]($func/smallcaps) function
- Improved default spacing for headings
- Added [`panic`]($func/panic) function
- Added [`clusters`]($type/string.clusters) and
  [`codepoints`]($type/string.codepoints)
  methods for strings
- Support for multiple authors in [`set document`]($func/document.author)
- Fixed crash when string is accessed at a position that is not a char boundary
- Fixed semicolon parsing in `[#var ;]`
- Fixed incremental parsing when inserting backslash at end of `[#"abc"]`
- Fixed names of a few font families
  (including Noto Sans Symbols and New Computer Modern families)
- Fixed autocompletion for font families
- Improved incremental compilation for user-defined functions

## February 15, 2023
- [Box]($func/box) and [block]($func/block) have gained `fill`, `stroke`,
  `radius`, and `inset` properties
- Blocks may now be explicitly sized, fixed-height blocks can still break
  across pages
- Blocks can now be configured to be [`breakable`]($func/block.breakable) or not
- [Numbering style]($func/enum.numbering) can now be configured for nested enums
- [Markers]($func/list.marker) can now be configured for nested lists
- The [`eval`]($func/eval) function now expects code instead of markup and
  returns an arbitrary value. Markup can still be evaluated by surrounding the
  string with brackets.
- PDFs generated by Typst now contain XMP metadata
- Link boxes are now disabled in PDF output
- Tables don't produce small empty cells before a pagebreak anymore
- Fixed raw block highlighting bug

## February 12, 2023
- Shapes, images, and transformations (move/rotate/scale/repeat) are now
  block-level. To integrate them into a paragraph, use a [`box`]($func/box) as
  with other elements.
- A colon is now required in an "everything" show rule: Write `{show: it => ..}`
  instead of `{show it => ..}`. This prevents intermediate states that ruin
  your whole document.
- Non-math content like a shape or table in a math formula is now centered
  vertically
- Support for widow and orphan prevention within containers
- Support for [RTL]($func/text.dir) in lists, grids, and tables
- Support for explicit `{auto}` sizing for boxes and shapes
- Support for fractional (i.e. `{1fr}`) widths for boxes
- Fixed bug where columns jump to next page
- Fixed bug where list items have no leading
- Fixed relative sizing in lists, squares and grid auto columns
- Fixed relative displacement in [`place`]($func/place) function
- Fixed that lines don't have a size
- Fixed bug where `{set document(..)}` complains about being after content
- Fixed parsing of `{not in}` operation
- Fixed hover tooltips in math
- Fixed bug where a heading show rule may not contain a pagebreak when an
  outline is present
- Added [`baseline`]($func/box.baseline) property on [`box`]($func/box)
- Added [`tg`]($func/math.op) and [`ctg`]($func/math.op) operators in math
- Added delimiter setting for [`cases`]($func/math.cases) function
- Parentheses are now included when accepting a function autocompletion

## February 2, 2023
- Merged text and math symbols, renamed a few symbols
  (including `infty` to `infinity` with the alias `oo`)
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

## January 30, 2023
[Go to the announcement blog post.](https://typst.app/blog/2023/january-update)
- New expression syntax in markup/math
  - Blocks cannot be directly embedded in markup anymore
  - Like other expressions, they now require a leading hashtag
  - More expressions available with hashtag, including literals
    (`[#"string"]`) as well as field access and method call
    without space: `[#emoji.face]`
- New import syntax
  - `[#import "module.typ"]` creates binding named `module`
  - `[#import "module.typ": a, b]` or `[#import "module.typ": *]` to import items
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
  - Variables and function calls directly in math (without hashtag) access this
    module instead of the global scope, but can also access local variables
  - Can be explicitly used in code, e.g. `[#set math.vec(delim: "[")]`
- Delimiter matching in math
   - Any opening delimiters matches any closing one
   - When matched, they automatically scale
   - To prevent scaling, escape them
   - To forcibly match two delimiters, use `lr` function
   - Line breaks may occur between matched delimiters
   - Delimiters may also be unbalanced
   - You can also use the `lr` function to scale the brackets
     (or just one bracket) to a specific size manually
- Multi-line math with alignment
  - The `\` character inserts a line break
  - The `&` character defines an alignment point
  - Alignment points also work for underbraces, vectors, cases, and matrices
  - Multiple alignment points are supported
- More capable math function calls
  - Function calls directly in math can now take code expressions with hashtag
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
  - Symbols and other content may now be used like a function, e.g. `[$zeta(x)$]`
  - Added Fira Math font, removed Noto Sans Math font
  - Support for alternative math fonts through
    `[#show math.formula: set text("Fira Math")]`
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
