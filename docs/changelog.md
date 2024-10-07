---
description: |
  Learn what has changed in the latest Typst releases and move your documents
  forward.
---

# Changelog

## Unreleased
### Highlights { #_ }
- Added support for multi-column floating [placement]($place.scope) and
  [figures]($figure.scope)
- Added support for automatic [line numbering]($par.line) (often used in
  academic papers)
- Typst's layout engine is now multithreaded. Typical speedups are 2-3x for
  larger documents. The multithreading operates on page break boundaries, so
  explicit page breaks are necessary for it to kick in.
- Paragraph justification was optimized with a new two-pass algorithm. Speedups
  are larger for shorter paragraphs and go up to 6x.
- Highly reduced PDF file sizes due to better font subsetting (thanks to
  [@LaurenzV](https://github.com/LaurenzV))
- Emoji are now exported properly in PDF
- Added initial support for PDF/A. For now, only the standard PDF/A-2b is
  supported, but more is planned for the future.
- Added various options for configuring the CLI's environment (fonts, package
  paths, etc.)
- Text show rules now match across multiple text elements
- Block-level equations can now break over multiple pages
- Fixed a bug where some fonts would not print correctly on professional
  printers
- Fixed a long-standing bug which could cause headings to be orphaned at the
  bottom of the page

### All changes { #_ }
- Layout
  - Added support for multi-column floating placement and figures via
    [`place.scope`] and [`figure.scope`].  Two-column documents should now
    prefer `{set page(columns: 2)}` over `{show: column.with(2)}` (see the [page
    setup guide]($guides/page-setup-guide/#columns)).
  - Added support for automatic [line numbering]($par.line) (often used in
    academic papers)
  - Added [`par.spacing`] property for configuring paragraph spacing. This
    should now be used instead of `{show par: set block(spacing: ..)}`
    (**Breaking change**)
  - Added [`block.sticky`] property which prevents a page break after a block
  - Added [`place.flush`] function which forces all floating figures to be
    placed before any further content
  - Added [`skew`] function
  - Added `{auto}` option for [`page.header`] and [`page.footer`] which results
    in an automatic header/footer based on the numbering (which was previously
    inaccessible after a change)
  - Added `gap` and `justify` parameters to [`repeat`] function
  - Added `width` and `height` parameters to the [`measure`] function to define
    the space in which the content should be measured. Especially useful in
    combination with [`layout`].
  - The height of a `block`, `image`, `rect`, `square`, `ellipse`, or `circle`
    can now be specified in [fractional units]($fraction)
  - The [`scale`] function now supports absolute lengths for `x`, `y`, `factor`.
    This way an element of unknown size can be scaled to a fixed size.
  - The values of `block.above` and `block.below` can now be retrieved in
    context expressions.
  - Fixed a bug which could cause headings to be orphaned at the bottom of the
    page
  - Fixed footnotes within breakable blocks appearing on the page where the
    breakable block ends instead of at the page where the footnote marker is
  - Fixed empty pages appearing when a [context] expression wraps whole pages
  - Fixed `{set block(spacing: x)}` behaving differently from
    `{set block(above: x, below: x)}`
  - Fixed behavior of [`rotate`] and [`scale`] with `{reflow: true}`
  - Fixed interaction of `{align(horizon)}` and `{v(1fr)}`
  - Fixed various bugs where floating placement would yield overlapping results
  - Fixed a bug where widow/orphan prevention would unnecessarily move text into
    the next column
  - Fixed [weak spacing]($h.weak) not being trimmed at the start and end of
    lines in a paragraph (only at the start and end of paragraphs)
  - Fixed interaction of weak page break and [`pagebreak.to`]
  - Fixed compilation output of a single weak page break
  - Fixed crash when [padding]($pad) by `{100%}`

- Text
  - Tuned hyphenation: It is less eager by default and hyphenations close to the
    edges of words are now discouraged more strongly
    (**May lead to larger layout reflows**)
  - New default font: Libertinus Serif. This is the maintained successor to the
    old default font Linux Libertine. (**May lead to smaller reflows**)
  - Setting the font to an unavailable family will now result in a warning
  - Implemented a new smart quote algorithm, fixing various bugs where smart
    quotes weren't all that smart
  - Added [`text.costs`] parameter for tweaking various parameters that affect
    the choices of the layout engine during text layout
  - Added `typm` highlighting mode for math in [raw blocks]($raw.lang)
  - Added basic i18n for Galician, Catalan, Latin, Icelandic, Hebrew
  - Implemented hyphenation duplication for Czech, Croatian, Lower Sorbian,
    Polish, Portuguese, Slovak, and Spanish.
  - The [`smallcaps`] function is now an element function and can thereby be
    used in show(-set) rules.
  - The [`raw.theme`] parameter can now be set to `{none}` to disable
    highlighting even in the presence of a language tag, and to `{auto}` to
    reset to the default
  - Multiple [stylistic sets]($text.stylistic-set) can now be enabled at once
  - Fixed the Chinese translation for "Equation"
  - Fixed that hyphenation could occur outside of words
  - Fixed incorrect layout of bidirectional text in edge cases
  - Fixed layout of paragraphs with explicit trailing whitespace
  - Fixed bugs related to empty paragraphs created via `#""`
  - Fixed accidental trailing spaces for line breaks immediately preceding an
    inline equation
  - Fixed [`text.historical-ligatures`] not working correctly
  - Fixed accidental repetition of Thai characters around line breaks in some
    circumstances
  - Fixed [smart quotes]($smartquote) for Swiss French
  - New font metadata exceptions for Archivo, Kaiti SC, and Kaiti TC
  - Updated bundled New Computer Modern fonts to version 6.0

- Math
  - Block-level equations can now break over multiple pages. This behavior can
    be disabled via `{show math.equation: set block(breakable: false)}`.
  - Matrix and vector sizing is now more consistent across different cell
    contents
  - Added [`stretch`]($math.stretch) function for manually or automatically
    stretching characters like arrows or parentheses horizontally or vertically
  - Improved layout of attachments on parenthesized as well as under- or
    overlined expressions
  - Improved layout of nested attachments resulting from code like
    `[#let a0 = $a_0$; $a0^1$]`
  - Improved layout of primes close to superscripts
  - Typst now makes use of math-specific height-dependent kerning information in
    some fonts for better attachment layout
  - The `floor` and `ceil` functions in math are now callable symbols, such that
    `[$ floor(x) = lr(floor.l x floor.r) $]`
  - The [`mat.delim`]($math.mat.delim), [`vec.delim`]($math.vec.delim), and
    [`cases.delim`]($math.cases.delim) parameters now allow any character that
    is considered a delimiter or "fence" (e.g. |) by Unicode. The
    `{delim: "||"}` notation is _not_ supported anymore and should be replaced
    by `{delim: bar.double}` (**Minor breaking change**)
  - Added [`vec.align`]($math.vec.align) and [`mat.align`]($math.mat.align)
    parameters
  - Added [`underparen`]($math.underparen), [`overparen`]($math.overparen),
    [`undershell`]($math.undershell), and [`overshell`]($math.overshell)
  - Added `~` shorthand for `tilde.op` in math mode (**Minor breaking change**)
  - Fixed baseline alignment of equation numbers
  - Fixed positioning of corner brackets (⌜, ⌝, ⌞, ⌟)
  - Fixed baseline of large roots
  - Fixed multiple minor layout bugs with attachments
  - Fixed that alignment points could affect line height in math
  - Fixed that spaces could show up between text and invisible elements like
    [`metadata`] in math
  - Fixed a crash with recursive show rules in math
  - Fixed [`lr.size`]($math.lr.size) not affecting characters enclosed in
    [`mid`]($math.mid) in some cases

- Introspection
  - Implemented a new system by which Typst tracks where elements end up on the
    pages. This may lead to subtly different behavior in introspections.
    (**Breaking change**)
  - Fixed various bugs with wrong counter behavior in complex layout
    situations, through a new, more principled implementation
  - Counter updates can now be before the first, in between, and after the last
    page when isolated by weak page breaks. This allows, for instance, updating
    a counter before the first page header and background.
  - Fixed incorrect [`here().position()`]($here) when [`place`] was used in a
    context expression
  - Fixed resolved positions of elements (in particular, headings) whose show
    rule emits an invisible element (like a state update) before a page break
  - Fixed behavior of stepping a counter at a deeper level than its current
    state has
  - Fixed citation formatting not working in table headers and a few other
    places
  - Displaying the footnote counter will now respect the footnote numbering
    style

- Model
  - Document set rules do not need to be at the very start of the document
    anymore. The only restriction is that they must not occur inside of layout
    containers.
  - The `spacing` property of [lists]($list.spacing),
    [enumerations]($enum.spacing), and [term lists]($terms.spacing) is now also
    respected for tight lists
  - Tight lists now only attach (with tighter spacing) to preceding paragraphs,
    not arbitrary blocks
  - The [`quote`] element is now locatable (can be used in queries)
  - The bibliography heading now uses `depth` instead of `level` so that its
    level can still be configured via a show-set rule
  - Added support for more [numbering] formats: Devanagari, Eastern Arabic,
    Bengali, and circled numbers
  - Added [`hanging-indent`]($heading.hanging-indent) parameter to heading
    function to tweak the appearance of multi-line headings and improved default
    appearance of multi-line headings
  - Improved handling of bidirectional text in outline entry
  - Fixed document set rules being ignored in an otherwise empty document
  - Fixed document set rules not being usable in context expressions
  - Fixed bad interaction between `{set document}` and `{set page}`
  - Fixed `{show figure: set align(..)}`. Since the default figure alignment is
    now a show-set rule, it is not revoked by `{show figure: it => it.body}`
    anymore. (**Minor breaking change**)
  - Fixed numbering of footnote references
  - Fixed spacing after bibliography heading

- Bibliography
  - The Hayagriva YAML `publisher` field can now accept a dictionary with a
    `location` key. The top-level `location` key is now primarily intended for
    event and item locations.
  - Multiple page ranges with prefixes and suffixes are now allowed
  - Added `director` and catch-all editor types to BibLaTeX parsing
  - Fixes for sorting of bibliography entries
  - Fixed pluralization of page range labels
  - Fixed sorting of citations by their number
  - Fixed how citation number ranges collapse
  - Fixed when the short form of a title is used
  - Fixed parsing of unbalanced dollars in BibLaTeX `url` field
  - Updated built-in citation styles

- Visualization
  - Added `fill-rule` parameter to [`path`]($path.fill-rule) and
    [`polygon`]($polygon.fill-rule) functions
  - Fixed color mixing and gradients for [Luma colors]($color.luma)
  - Fixed conversion from Luma to CMYK colors
  - Fixed offset gradient strokes in PNG export
  - Fixed unintended cropping of some SVGs
  - SVGs with foreign objects now produce a warning as they will likely not
    render correctly in Typst

- Syntax
  - Added support for nested imports like `{import "file.typ": module.item}`
  - Added support for parenthesized imports like
    `{import "file.typ": (a, b, c)}`. With those, the import list can break over
    multiple lines.
  - Fixed edge case in parsing of reference syntax
  - Fixed edge case in parsing of heading, list, enum, and term markers
    immediately followed by comments
  - Fixed rare crash in parsing of parenthesized expressions

- Scripting
  - Added new fixed-point [`decimal`] number type for when highly precise
    arithmetic is needed, such as for finance
  - Added `std` module for accessing standard library definitions even when a
    variable with the same name shadows/overwrites it
  - Added [`array.to-dict`], [`array.reduce`], [`array.windows`] methods
  - Added `exact` argument to [`array.zip`]
  - Added [`arguments.at`] method
  - Added [`int.from-bytes`], [`int.to-bytes`], [`float.from-bytes`], and
    [`float.to-bytes`]
  - The `digits` parameter of [`calc.round`] no longer accepts negative integers
    (**Minor breaking change**)
  - Conversions from [`int`] to [`float`] will now error instead of saturating
    if the float is too large (**Minor breaking change**)
  - Added `float.nan` and `float.inf`, removed `calc.nan`
    (**Minor breaking change**)
  - Certain symbols are now generally callable like functions and not only
    specifically in math. Examples are accents or [`floor`]($math.floor) and
    [`ceil`]($math.ceil).
  - Improved [`repr`] of relative values, sequences, infinities, NaN,
    `{type(none)}` and `{type(auto)}`
  - Fixed crash on whole packages (rather than just files) cyclically importing
    each other
  - Fixed behavior of [`calc.round`] on integers when a non-zero value is
    provided for `digits`

- Styling
  - Text show rules now match across multiple text elements
  - The string `{"}` in a text show rule now matches smart quotes
  - Fixed a long-standing styling bug where the header and footer would
    incorrectly inherit styles from a lone element on the page (e.g. a heading)
  - Fixed `{set page}` not working directly after a counter/state update
  - Page fields configured via an explicit `{page(..)[..]}` call can now be
    properly retrieved in context expressions

- Export
  - Highly reduced PDF file sizes due to better font subsetting
  - Emoji are now exported properly in PDF
  - Added initial support for PDF/A. For now, only the standard PDF/A-2b is
    supported, but more is planned for the future. Enabled via `--pdf-standard
    a-2b` in the CLI and via the UI in File > Export as > PDF in the web app.
  - Setting [`page.fill`] to `{none}` will now lead to transparent pages instead
    of white ones in PNG and SVG. The new default of `{auto}` means transparent
    for PDF and white for PNG and SVG.
  - Improved text copy-paste from PDF in complex scenarios
  - Exported SVGs now contain the `data-typst-label` attribute on groups
    resulting from labelled [boxes]($box) and [blocks]($block)
  - Fixed a bug where some fonts would not print correctly on professional
    printers
  - Fixed a bug where transparency could leak from one PDF object to another
  - Fixed a bug with CMYK gradients in PDF
  - Fixed various bugs with export of Oklab gradients in PDF
  - Two small fixes for PDF standard conformance

- Performance
  - Typst's layout engine is now multithreaded. Typical speedups are 2-3x for
    larger documents. The multithreading operates on page break boundaries, so
    explicit page breaks are necessary for it to kick in.
  - Paragraph justification was optimized with a new two-pass algorithm.
    Speedups are larger for shorter paragraphs and range from 1-6x.

- Command Line Interface
  - Added `--pages` option to select specific page ranges to export
  - Added `--package-path` and `--package-cache-path` as well as
    `TYPST_PACKAGE_PATH` and `TYPST_PACKAGE_CACHE_PATH` environment variables
    for configuring where packages are loaded from and cached in, respectively
  - Added `--ignore-system-fonts` flag to disable system fonts fully for better
    reproducibility
  - Added `--make-deps` argument for outputting the dependencies of the current
    compilation as a Makefile
  - Added `--pretty` option to `typst query`, with the default now being to
    minify
  - Added `--backup-path` to `typst update` to configure where the previous
    version is backed up
  - The document can now be written to stdout by passing `-` as the output
    filename (for PDF or single-page image export)
  - Typst will now emit a proper error message instead of failing silently when
    the certificate specified by `--cert` or `TYPST_CERT` could not be loaded
  - The CLI now respects the `SOURCE_DATE_EPOCH` environment variable for better
    reproducibility
  - When exporting multiple images, you can now use `{t}` (total pages), `{p}`
    (current page), and `{0p}` (zero-padded current page, same as current `{n}`)
    in the output path
  - The input and output paths now allow non-UTF-8 values
  - Times are now formatted more consistently across the CLI
  - Fixed a bug related to the `--open` flag
  - Fixed path completions for `typst` not working in zsh

- Tooling & Diagnostics
  - The "compiler" field for specifying the minimum Typst version required by a
    package now supports imprecise bounds like 0.11 instead of 0.11.0
  - Added warning when a label is ignored by Typst because no preceding
    labellable element exists
  - Added hint when trying to apply labels in code mode
  - Added hint when trying to call a standard library function that has been
    shadowed/overwritten by a local definition
  - Added hint when trying to set both the language and the region in the `lang`
    parameter
  - Added hints when trying to compile non-Typst files (e.g. after having typed
    `typst c file.pdf` by accident)
  - Added hint when a string is used where a label is expected
  - Added hint when a stray end of a block comment (`*/`) is encountered
  - Added hints when destructuring arrays with the wrong number of elements
  - Improved error message when trying to use a keyword as an identifier in a
    let binding
  - Improved error messages when accessing nonexistent fields
  - Improved error message when a package exists, but not the specified version
  - Improved hints for unknown variables
  - Improved hint when trying to convert a length with non-zero em component to
    an absolute unit
  - Fixed a crash that could be triggered by certain hover tooltips
  - Fixed an off-by-one error in to-source jumps when first-line-indent is
    enabled
  - Fixed suggestions for `.` after the end of an inline code expressions
  - Fixed autocompletions being duplicated in a specific case

- Symbols
  - New: `parallelogram`, `original`, `image`, `crossmark`, `rest`, `natural`,
    `flat`, `sharp`, `tiny`, `miny`, `copyleft`, `trademark`, `emoji.beet`,
    `emoji.fingerprint`, `emoji.harp`, `emoji.shovel`, `emoji.splatter`,
    `emoji.tree.leafless`,
  - New variants: `club.stroked`, `diamond.stroked`, `heart.stroked`,
    `spade.stroked`, `gt.neq`, `lt.neq`, `checkmark.heavy`, `paren.double`,
    `brace.double`, `shell.double`, `arrow.turn`, `plus.double`, `plus.triple`,
    `infinity.bar`, `infinity.incomplete`, `infinity.tie`, `multimap.double`,
    `ballot.check`, `ballot.check.heavy`, `emptyset.bar`, `emptyset.circle`,
    `emptyset.arrow.l`, `emptyset.arrow.r`, `parallel.struck`, `parallel.eq`,
    `parallel.equiv`, `parallel.slanted`, `parallel.tilde`, `angle.l.curly`,
    `angle.l.dot`, `angle.r.curly`, `angle.r.dot`, `angle.oblique`, `angle.s`,
    `em.two`, `em.three`
  - Renamed: `turtle` to `shell`, `notes` to `note`, `ballot.x` to
    `ballot.cross`, `succ.eq` to `succ.curly.eq`, `prec.eq` to `prec.curly.eq`,
    `servicemark` to `trademark.service`, `emoji.face.tired` to
    `emoji.face.distress` (**Breaking change**)
  - Changed codepoint: `prec.eq`, `prec.neq`, `succ.eq`, `succ.neq`, `triangle`
    from ▷ to △, `emoji.face.tired` (**Breaking change**)
  - Removed: `lt.curly` in favor of `prec`, `gt.curly` in favor of `succ`
    (**Breaking change**)

- Deprecations
  - [`counter.display`] without an established context
  - [`counter.final`] with a location
  - [`state.final`] with a location
  - [`state.display`]
  - [`query`] with a location as the second argument
  - [`locate`] with a callback function
  - [`measure`] with styles
  - [`style`]

- Development
  - Added `typst-kit` crate which provides useful APIs for `World` implementors
  - Added go-to-definition API in `typst-ide`
  - Added package manifest parsing APIs to `typst-syntax`
  - As the compiler is now capable of multithreading, `World` implementations
    must satisfy `Send` and `Sync`
  - Changed signature of `World::main` to allow for the scenario where the main
    file could not be loaded
  - Removed `Tracer` in favor of `Warned<T>` and `typst::trace` function
  - The `xz2` dependency used by the self-updater is now statically linked
  - The Dockerfile now has an `ENTRYPOINT` directive

## Version 0.11.1 (May 17, 2024) { #v0.11.1 }
- Security
  - Fixed a vulnerability where image files at known paths could be embedded
    into the PDF even if they were outside of the project directory

- Bibliography
  - Fixed et-al handling in subsequent citations
  - Fixed suppression of title for citations and bibliography references with no
    author
  - Fixed handling of initials in citation styles without a delimiter
  - Fixed bug with citations in footnotes

- Text and Layout
  - Fixed interaction of [`first-line-indent`]($par.first-line-indent) and
    [`outline`]
  - Fixed compression of CJK punctuation marks at line start and end
  - Fixed handling of [rectangles]($rect) with negative dimensions
  - Fixed layout of [`path`] in explicitly sized container
  - Fixed broken [`raw`] text in right-to-left paragraphs
  - Fixed tab rendering in `raw` text with language `typ` or `typc`
  - Fixed highlighting of multi-line `raw` text enclosed by single backticks
  - Fixed indentation of overflowing lines in `raw` blocks
  - Fixed extra space when `raw` text ends with a backtick

- Math
  - Fixed broken [equations]($math.equation) in right-to-left paragraphs
  - Fixed missing [blackboard bold]($math.bb) letters
  - Fixed error on empty arguments in 2D math argument list
  - Fixed stretching via [`mid`]($math.mid) for various characters
  - Fixed that alignment points in equations were affected by `{set align(..)}`

- Export
  - Fixed [smart quotes]($smartquote) in PDF outline
  - Fixed [patterns]($pattern) with spacing in PDF
  - Fixed wrong PDF page labels when [page numbering]($page.numbering) was
    disabled after being previously enabled

- Scripting
  - Fixed overflow for large numbers in external data files (by converting to
    floats instead)
  - Fixed [`{str.trim(regex, at: end)}`]($str.trim) when the whole string is
    matched

- Miscellaneous
  - Fixed deformed strokes for specific shapes and thicknesses
  - Fixed newline handling in code mode: There can now be comments within
    chained method calls and between an `if` branch and the `else` keyword
  - Fixed inefficiency with incremental reparsing
  - Fixed autocompletions for relative file imports
  - Fixed crash in autocompletion handler
  - Fixed a bug where the path and entrypoint printed by `typst init` were not
    properly escaped
  - Fixed various documentation errors

<contributors from="v0.11.0" to="v0.11.1" />

## Version 0.11.0 (March 15, 2024) { #v0.11.0 }
- Tables (thanks to [@PgBiel](https://github.com/PgBiel))
  - Tables are now _much_ more flexible, read the new
    [table guide]($guides/table-guide) to get started
  - Added [`table.cell`] element for per-cell configuration
  - Cells can now span multiple [columns]($table.cell.colspan) or
    [rows]($table.cell.rowspan)
  - The [stroke]($table.cell.stroke) of individual cells can now be customized
  - The [`align`]($table.align) and [`inset`]($table.inset) arguments of the
    table function now also take `{(x, y) => ..}` functions
  - Added [`table.hline`] and [`table.vline`] for convenient line customization
  - Added [`table.header`] element for table headers that repeat on every page
  - Added [`table.footer`] element for table footers that repeat on every page
  - All the new table functionality is also available for [grids]($grid)
  - Fixed gutter-related bugs

- Templates
  - You can now use template packages to get started with new projects. Click
    _Start from template_ on the web app's dashboard and choose your preferred
    template or run the `typst init <template>` command in the CLI. You can
    [browse the available templates here]($universe/search/?kind=templates).
  - Switching templates after the fact has become easier. You can just import a
    styling function from a different template package.
  - Package authors can now submit their own templates to the
    [package repository](https://github.com/typst/packages). Share a template
    for a paper, your institution, or an original work to help the community get
    a head start on their projects.
  - Templates and packages are now organized by category and discipline. Filter
    packages by either taxonomy in the _Start from template_ wizard. If you are
    a package author, take a look at the new documentation for
    [categories](https://github.com/typst/packages/blob/main/CATEGORIES.md) and
    [disciplines](https://github.com/typst/packages/blob/main/DISCIPLINES.md).

- Context
  - Added _context expressions:_ Read the chapter on [context] to get started
  - With context, you can access settable properties, e.g. `{context text.lang}`
    to access the language set via `{set text(lang: "..")}`
  - The following existing functions have been made contextual: [`query`],
    [`locate`], [`measure`], [`counter.display`], [`counter.at`],
    [`counter.final`], [`state.at`], and [`state.final`]
  - Added contextual methods [`counter.get`] and [`state.get`] to retrieve the
    value of a counter or state in the current context
  - Added contextual function [`here`] to retrieve the [location] of the current
    context
  - The [`locate`] function now returns the location of a selector's unique
    match. Its old behavior has been replaced by context expressions and only
    remains temporarily available for compatibility.
  - The [`counter.at`] and [`state.at`] methods are now more flexible: They
    directly accept any kind of [locatable]($location/#locatable) selector with
    a unique match (e.g. a label) instead of just locations
  - When context is available, [`counter.display`] now directly returns the
    result of applying the numbering instead of yielding opaque content. It
    should not be used anymore without context. (Deprecation planned)
  - The [`state.display`] function should not be used anymore, use [`state.get`]
    instead (Deprecation planned)
  - The `location` argument of [`query`], [`counter.final`], and
    [`state.final`] should not be used anymore (Deprecation planned)
  - The [`styles`]($measure.styles) argument of the `measure` function should
    not be used anymore (Deprecation planned)
  - The [`style`] function should not be used anymore, use context instead
    (Deprecation planned)
  - The correct context is now also provided in various other places where it is
    available, e.g. in show rules, layout callbacks, and numbering functions
    in the outline

- Styling
  - Fixed priority of multiple [show-set rules]($styling/#show-rules): They now
    apply in the same order as normal set rules would
  - Show-set rules on the same element
    (e.g. `{show heading.where(level: 1): set heading(numbering: "1.")}`) now
    work properly
  - Setting properties on an element within a transformational show rule (e.g.
    `{show heading: it => { set heading(..); it }}`) is **not** supported
    anymore (previously it also only worked sometimes); use show-set rules
    instead (**Breaking change**)
  - Text show rules that match their own output now work properly
    (e.g. `` {show "cmd": `cmd`} ``)
  - The elements passed to show rules and returned by queries now contain all
    fields of their respective element functions rather than just specific ones
  - All settable properties can now be used in [where]($function.where)
    selectors
  - [And]($selector.and) and [or]($selector.or) selectors can now be used with
    show rules
  - Errors within show rules and context expressions are now ignored in all but
    the last introspection iteration, in line with the behavior of the old
    [`locate`]
  - Fixed a bug where document set rules were allowed after content

- Layout
  - Added `reflow` argument to [`rotate`]($rotate) and [`scale`]($scale) which
    lets them affect the layout
  - Fixed a bug where [floating placement]($place.float) or
    [floating figures]($figure.placement) could end up out of order
  - Fixed overlap of text and figure for full-page floating figures
  - Fixed various cases where the [`hide`] function didn't hide its contents
    properly
  - Fixed usage of [`h`] and [`v`] in [stacks]($stack)
  - Invisible content like a counter update will no longer force a visible
    block for just itself
  - Fixed a bug with horizontal spacing followed by invisible content (like a
    counter update) directly at the start of a paragraph

- Text
  - Added [`stroke`]($text.stroke) property for text
  - Added basic i18n for Serbian and Catalan
  - Added support for contemporary Japanese [numbering] method
  - Added patches for various wrong metadata in specific fonts
  - The [text direction]($text.dir) can now be overridden within a paragraph
  - Fixed Danish [smart quotes]($smartquote)
  - Fixed font fallback next to a line break
  - Fixed width adjustment of JIS-style Japanese punctuation
  - Fixed Finnish translation of "Listing"
  - Fixed Z-ordering of multiple text decorations (underlines, etc.)
  - Fixed a bug due to which text [features]($text.features) could not be
    overridden in consecutive set rules

- Model
  - Added [`depth`]($heading.depth) and [`offset`]($heading.offset) arguments to
    heading to increase or decrease the heading level for a bunch of content;
    the heading syntax now sets `depth` rather than `level`
    (**Breaking change**)
  - List [markers]($list.marker) now cycle by default
  - The [`quote`] function now more robustly selects the correct quotes based on
    language and nesting
  - Fixed indent bugs related to the default show rule of [terms]

- Math
  - Inline equations now automatically linebreak at appropriate places
  - Added [`number-align`]($math.equation.number-align) argument to equations
  - Added support for adjusting the [`size`]($math.accent.size) of accents
    relative to their base
  - Improved positioning of accents
  - [Primes]($math.primes) are now always attached as [scripts]($math.scripts)
    by default
  - Exposed [`math.primes`] element which backs the `[$f'$]` syntax in math
  - Math mode is not affected by [`strong`] and [`emph`] anymore
  - Fixed [`attach`]($math.attach) under [fractions]($math.frac)
  - Fixed that [`math.class`] did not affect smart limit placement
  - Fixed weak spacing in [`lr`]($math.lr) groups
  - Fixed layout of large operators for Cambria Math font
  - Fixed math styling of Hebrew symbol codepoints

- Symbols
  - Added `gradient` as an alias for `nabla`
  - Added `partial` as an alias for `diff`, `diff` will be deprecated in the
    future
  - Added `colon.double`, `gt.approx`, `gt.napprox`, `lt.approx`, and
    `lt.napprox`
  - Added `arrow.r.tilde` and `arrow.l.tilde`
  - Added `tilde.dot`
  - Added `forces` and `forces.not`
  - Added `space.nobreak.narrow`
  - Added `lrm` (Left-to-Right Mark) and `rlm` (Right-to-Left Mark)
  - Fixed `star.stroked` symbol (which previously had the wrong codepoint)

- Scripting
  - Arrays can now be compared lexicographically
  - Added contextual method [`to-absolute`]($length.to-absolute) to lengths
  - Added [`calc.root`]($calc.root)
  - Added [`int.signum`] and [`float.signum`] methods
  - Added [`float.is-nan`] and [`float.is-infinite`] methods
  - Added [`int.bit-not`], [`int.bit-and`], [`int.bit-or`], [`int.bit-xor`],
    [`int.bit-lshift`], and [`int.bit-rshift`] methods
  - Added [`array.chunks`] method
  - A module can now be converted to a dictionary with the
    [dictionary constructor]($dictionary/#constructor) to access its contents
    dynamically
  - Added [`row-type`]($csv.row-type) argument to `csv` function to configure
    how rows will be represented
  - [XML parsing]($xml) now allows DTDs (document type definitions)
  - Improved formatting of negative numbers with [`str`]($str) and
    [`repr`]($repr)
  - For loops can now iterate over [bytes]
  - Fixed a bug with pattern matching in for loops
  - Fixed a bug with labels not being part of [`{.fields()}`]($content.fields)
    dictionaries
  - Fixed a bug where unnamed argument sinks wouldn't capture excess arguments
  - Fixed typo in `repr` output of strokes

- Syntax
  - Added support for nested [destructuring patterns]($scripting/#bindings)
  - Special spaces (like thin or non-breaking spaces) are now parsed literally
    instead of being collapsed into normal spaces (**Breaking change**)
  - Korean text can now use emphasis syntax without adding spaces
    (**Breaking change**)
  - The token [`context`] is now a keyword and cannot be used as an identifier
    anymore (**Breaking change**)
  - Nested line comments aren't allowed anymore in block comments
    (**Breaking change**)
  - Fixed a bug where `x.)` would be treated as a field access
  - Text elements can now span across curly braces in markup
  - Fixed silently wrong parsing when function name is parenthesized
  - Fixed various bugs with parsing of destructuring patterns, arrays, and
    dictionaries

- Tooling & Diagnostics
  - Click-to-jump now works properly within [`raw`] text
  - Added suggestion for accessing a field if a method doesn't exist
  - Improved hint for calling a function stored in a dictionary
  - Improved errors for mutable accessor functions on arrays and dictionaries
  - Fixed error message when calling constructor of type that doesn't have one
  - Fixed confusing error message with nested dictionaries for strokes on
    different sides
  - Fixed autocompletion for multiple packages with the same name from different
    namespaces

- Visualization
  - The [`image`] function doesn't upscale images beyond their natural size
    anymore
  - The [`image`] function now respects rotation stored in EXIF metadata
  - Added support for SVG filters
  - Added alpha component to [`luma`]($color.luma) colors
  - Added [`color.transparentize`] and [`color.opacify`] methods
  - Improved [`color.negate`] function
  - Added [`stroke`]($highlight.stroke) and [`radius`]($highlight.radius)
    arguments to `highlight` function
  - Changed default [`highlight`] color to be transparent
  - CMYK to RGB conversion is now color-managed
  - Fixed crash with gradients in Oklch color space
  - Fixed color-mixing for hue-based spaces
  - Fixed bugs with color conversion
  - SVG sizes are not rounded anymore, preventing slightly wrong aspect ratios
  - Fixed a few other SVG-related bugs
  - [`color.components`] doesn't round anything anymore

- Export
  - PDFs now contain named destinations for headings derived from their labels
  - The internal PDF structure was changed to make it easier for external tools
    to extract or modify individual pages, avoiding a bug with Typst PDFs in
    Apple Preview
  - PDFs produced by Typst should now be byte-by-byte reproducible when
    `{set document(date: none)}` is set
  - Added missing flag to PDF annotation
  - Fixed multiple bugs with gradients in PDF export
  - Fixed a bug with patterns in PDF export
  - Fixed a bug with embedding of grayscale images in PDF export
  - Fixed a bug with To-Unicode mapping of CFF fonts in PDF export
  - Fixed a bug with the generation of the PDF outline
  - Fixed a sorting bug in PDF export leading to non-reproducible output
  - Fixed a bug with transparent text in PNG export
  - Exported SVG files now include units in their top-level `width` and `height`

- Command line interface
  - Added support for passing [inputs]($category/foundations/sys) via a CLI flag
  - When passing the filename `-`, Typst will now read input from stdin
  - Now uses the system-native TLS implementation for network fetching which
    should be generally more robust
  - Watch mode will now properly detect when a previously missing file is
    created
  - Added `--color` flag to configure whether to print colored output
  - Fixed user agent with which packages are downloaded
  - Updated bundled fonts to the newest versions

- Development
  - Added `--vendor-openssl` to CLI to configure whether to link OpenSSL
    statically instead of dynamically (not applicable to Windows and Apple
    platforms)
  - Removed old tracing (and its verbosity) flag from the CLI
  - Added new `--timings` flag which supersedes the old flamegraph profiling in
    the CLI
  - Added minimal CLI to `typst-docs` crate for extracting the language and
    standard library documentation as JSON
  - The `typst_pdf::export` function's `ident` argument switched from `Option`
    to `Smart`. It should only be set to `Smart::Custom` if you can provide
    a stable identifier (like the web app can). The CLI sets `Smart::Auto`.

<contributors from="v0.10.0" to="v0.11.0" />

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
  - Fixed behavior of alphanumeric style
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
  - Fixed [CJK-Latin-spacing]($text.cjk-latin-spacing) before line breaks and in
    [`locate`] calls
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
  - The [`stroke`] type now has a constructor that converts a value to a stroke
    or creates one from its parts
  - Added constructor for [`arguments`] type
  - Added [`calc.div-euclid`]($calc.div-euclid) and
    [`calc.rem-euclid`]($calc.rem-euclid) functions
  - Fixed equality of [`arguments`]
  - Fixed [`repr`]of [`cmyk`]($color.cmyk) colors
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
  - Fixed selectors on [quote] elements
  - Fixed parsing of `[#return]` expression in markup
  - Fixed bug where inline equations were displayed in equation outlines
  - Fixed potential CRLF issue in [`raw`] blocks
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
  - The [`cite`] function now takes only a single label/key instead of allowing
    multiple. Adjacent citations are merged and formatted according to the
    citation style's rules automatically. This works both with the reference
    syntax and explicit calls to the `cite` function. (**Breaking change**)
  - The `cite` function now takes a [label] instead of a string
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
  - Added `background` parameter to [`overline`], [`underline`], and [`strike`]
    functions
  - Fixed inaccurate color embedding in PDFs
  - Fixed ICC profile handling for images embedded in PDFs

- Text and Layout
  - Added support for automatically adding proper
    [spacing]($text.cjk-latin-spacing) between CJK and Latin text (enabled by
    default)
  - Added support for automatic adjustment of more CJK punctuation
  - Added [`quote`] element for inserting inline and block quotes with optional
    attributions
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
  - Fixed behavior of [`to`]($pagebreak.to) argument on `pagebreak` function
  - Fixed `{set align(..)}` for equations
  - Fixed spacing around [placed]($place) elements
  - Fixed coalescing of [`above`]($block.above) and [`below`]($block.below)
    spacing if given in em units and the font sizes differ
  - Fixed handling of `extent` parameter of [`underline`], [`overline`], and
    [`strike`] functions
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
  - Added [`version`] type and `sys.version` constant specifying the current
    compiler version. Can be used to gracefully support multiple versions.
  - The U+2212 MINUS SIGN is now used when displaying a numeric value, in the
    [`repr`] of any numeric value and to replace a normal hyphen in text mode
    when before a digit. This improves, in particular, how negative integer
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
    - A [type] is now itself a value
    - Some types can be called like functions (those that have a constructor),
      e.g. [`int`] and [`str`]
    - Type checks are now of the form `{type(10) == int}` instead of the old
      `{type(10) == "integer"}`. [Compatibility]($type/#compatibility) with the
      old way will remain for a while to give package authors time to upgrade,
      but it will be removed at some point.
    - Methods are now syntax sugar for calling a function scoped to a type,
      meaning that `{"hello".len()}` is equivalent to `{str.len("hello")}`
  - Added support for [`import`]($scripting/#modules) renaming with `as`
  - Added a [`duration`] type
  - Added support for [CBOR]($cbor) encoding and decoding
  - Added encoding and decoding functions from and to bytes for data formats:
    [`json.decode`]($json.decode), [`json.encode`]($json.encode), and similar
    functions for other formats
  - Added [`array.intersperse`]($array.intersperse) function
  - Added [`str.rev`]($str.rev) function
  - Added `calc.tau` constant
  - Made [bytes] joinable and addable
  - Made [`array.zip`]($array.zip) function variadic
  - Fixed bug with [`eval`] when the `mode` was set to `{"math"}`
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
  - Added [`highlight`] function for highlighting text with a
    background color
  - Added [`polygon.regular`]($polygon.regular) function for drawing a regular
    polygon
  - Added support for tabs in [`raw`] elements alongside
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
  - Added `mode` and `scope` arguments to [`eval`] function
  - Added [`bytes`] type for holding large byte buffers
    - Added [`encoding`]($read.encoding) argument to read function to read a
      file as bytes instead of a string
    - Added [`image.decode`]($image.decode) function for decoding an image
      directly from a string or bytes
    - Added [`bytes`] function for converting a string or an array of integers
      to bytes
    - Added [`array`] function for converting bytes to an array of integers
    - Added support for converting bytes to a string with the [`str`] function

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
  - Fixed autocompletion for `for` loops

- Command line interface
  - Added `typst query` subcommand to execute a
    [query]($reference/introspection/query/#command-line-queries) on the command
    line
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
  - Added [`metadata`] function for exposing an arbitrary value to the
    introspection system
  - Fixed that a [`state`] was identified by the pair `(key, init)` instead of
    just its `key`
  - Improved indent logic of [enumerations]($enum). Instead of requiring at
    least as much indent as the end of the marker, they now require only one
    more space indent than the start of the marker. As a result, even long
    markers like `12.` work with just 2 spaces of indent.
  - Fixed bug with indent logic of [`raw`] blocks
  - Fixed a parsing bug with dictionaries

- Development
  - Extracted parser and syntax tree into `typst-syntax` crate
  - The `World::today` implementation of Typst dependents may need fixing if
    they have the same [bug](https://github.com/typst/typst/issues/1842) that
    the CLI world had

<contributors from="v0.6.0" to="v0.7.0" />

## Version 0.6.0 (June 30, 2023) { #v0.6.0 }
- Package Management
  - Typst now has built-in [package management]($scripting/#packages)
  - You can import [published]($universe) community packages or create and use
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
  - Added some missing syntaxes for [`raw`] highlighting
  - Improved rendering of rotated images in PNG export and web app
  - Made [footnotes]($footnote) reusable and referenceable
  - Fixed bug with citations and bibliographies in [`locate`]
  - Fixed inconsistent tense in documentation

- Development
  - Added [contribution guide](https://github.com/typst/typst/blob/main/CONTRIBUTING.md)
  - Reworked `World` interface to accommodate for package management and make
    it a bit simpler to implement _(Breaking change for implementors)_

<contributors from="v0.5.0" to="v0.6.0" />

## Version 0.5.0 (June 9, 2023) { #v0.5.0 }
- Text and Layout
  - Added [`raw`] syntax highlighting for many more languages
  - Added support for Korean [numbering]
  - Added basic i18n for a few more languages (NL, SV, DA)
  - Improved line breaking for East Asian languages
  - Expanded functionality of outline [`indent`]($outline.indent) property
  - Fixed footnotes in columns
  - Fixed page breaking bugs with [footnotes]($footnote)
  - Fixed bug with handling of footnotes in lists, tables, and figures
  - Fixed a bug with CJK punctuation adjustment
  - Fixed a crash with rounded rectangles
  - Fixed alignment of [`line`] elements

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
  - Added new [`datetime`] type and [`datetime.today`]($datetime.today) to
    retrieve the current date
  - Added [`str.from-unicode`]($str.from-unicode) and
    [`str.to-unicode`]($str.to-unicode) functions
  - Added [`fields`]($content.fields) method on content
  - Added `base` parameter to [`str`] function
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
  - Now sets the correct exit when invoked with a nonexistent file
  - Now ignores UTF-8 BOM in Typst files

- Miscellaneous Improvements
  - Improved errors for mismatched delimiters
  - Improved error message for failed length comparisons
  - Fixed a bug with images not showing up in Apple Preview
  - Fixed multiple bugs with the PDF outline
  - Fixed citations and other searchable elements in [`hide`]
  - Fixed bugs with [reference supplements]($ref.supplement)
  - Fixed Nix flake

<contributors from="v0.4.0" to="v0.5.0" />

## Version 0.4.0 (May 20, 2023) { #v0.4.0 }
- Footnotes
  - Implemented support for footnotes
  - The [`footnote`] function inserts a footnote
  - The [`footnote.entry`]($footnote.entry) function can be used to customize
    the footnote listing
  - The `{"chicago-notes"}` [citation style]($cite.style) is now available

- Documentation
  - Added a [Guide for LaTeX users]($guides/guide-for-latex-users)
  - Now shows default values for optional arguments
  - Added richer outlines in "On this Page"
  - Added initial support for search keywords: "Table of Contents" will now find
    the [outline] function. Suggestions for more keywords are welcome!
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
    for instance, is part of the [`assert`] function's scope. Note that function
    scopes are currently only available for built-in functions.
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
    behavior. It will remain available as `mod` until the next update as a
    grace period.
  - A lone underscore is not a valid identifier anymore, it can now only be used
    in patterns
  - Removed `before` and `after` arguments from [`query`]. This is now handled
    through flexible [selectors]($selector) combinator methods
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
  - Added [`toml`] function for loading data from a TOML file
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
  - Added [`selector`] function to turn a selector-like value into a selector on
    which combinator methods can be called
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
  - Added [`path`] function for drawing Bézier paths
  - Added [`layout`] function to access the size of the surrounding page or
    container
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
  - Fixed bug where math could not be hidden with [`hide`]
  - Fixed sizing issues with box, block, and shapes
  - Fixed some translations
  - Fixed inversion of "R" in [`cal`]($math.cal) and [`frak`]($math.frak) styles
  - Fixed some styling issues in math
  - Fixed supplements of references to headings
  - Fixed syntax highlighting of identifiers in certain scenarios
  - [Ratios]($ratio) can now be multiplied with more types and be converted to
    [floats]($float) with the [`float`] function

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
  - Figures now automatically detect their content and adapt their behavior.
    Figures containing tables, for instance, are automatically prefixed with
    "Table X" and have a separate counter
  - The figure's supplement (e.g. "Figure" or "Table") can now be customized
  - In addition, figures can now be completely customized because the show rule
    gives access to the automatically resolved kind, supplement, and counter

- Bibliography improvements
  - The [`bibliography`] now also accepts multiple bibliography paths (as an
    array)
  - Parsing of BibLaTeX files is now more permissive (accepts non-numeric
    edition, pages, volumes, dates, and Jabref-style comments; fixed
    abbreviation parsing)
  - Labels and references can now include `:` and `.` except at the end
  - Fixed APA bibliography ordering

- Drawing additions
  - Added [`polygon`] function for drawing polygons
  - Added support for clipping in [boxes]($box.clip) and [blocks]($block.clip)

- Command line interface
  - Now returns with non-zero status code if there is an error
  - Now watches the root directory instead of the current one
  - Now puts the PDF file next to input file by default
  - Now accepts more kinds of input files (e.g. `/dev/stdin`)
  - Added `--open` flag to directly open the PDF

- Miscellaneous improvements
  - Added [`yaml`] function to load data from YAML files
  - Added basic i18n for a few more languages (IT, RU, ZH, FR, PT)
  - Added numbering support for Hebrew
  - Added support for [integers]($int) with base 2, 8, and 16
  - Added symbols for double bracket and laplace operator
  - The [`link`] function now accepts [labels]($label)
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
  - Fixed crash when [`symbol`] function was called without arguments
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
    [`location`] type
  - Added symbols for double, triple, and quadruple dot accent
  - Added smart quotes for Norwegian Bokmål
  - Added Nix flake
  - Fixed bibliography ordering in IEEE style
  - Fixed parsing of decimals in math: `[$1.2/3.4$]`
  - Fixed parsing of unbalanced delimiters in fractions: `[$1/(2 (x)$]`
  - Fixed unexpected parsing of numbers as enumerations, e.g. in `[1.2]`
  - Fixed combination of page fill and header
  - Fixed compiler crash if [`repeat`] is used in page with automatic width
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
  - [`counter`] function
    - Access and modify counters for pages, headings, figures, and equations
    - Define and use your own custom counters
    - Time travel: Find out what the counter value was or will be at some other
      point in the document (e.g. when you're building a list of figures, you
      can determine the value of the figure counter at any given figure).
    - Counters count in layout order and not in code order
  - [`state`] function
    - Manage arbitrary state across your document
    - Time travel: Find out the value of your state at any position in the
      document
    - State is modified in layout order and not in code order
  - [`query`] function
    - Find all occurrences of an element or a label, either in the whole
      document or before/after some location
    - Link to elements, find out their position on the pages and access their
      fields
    - Example use cases: Custom list of figures or page header with current
      chapter title
  - [`locate`] function
    - Determines the location of itself in the final layout
    - Can be accessed to get the `page` and `x`, `y` coordinates
    - Can be used with counters and state to find out their values at that
      location
    - Can be used with queries to find elements before or after its location

- New [`measure`] function
  - Measure the layouted size of elements
  - To be used in combination with the new [`style`] function that lets you
    generate different content based on the style context something is inserted
    into (because that affects the measured size of content)

- Exposed content representation
  - Content is not opaque anymore
  - Content can be compared for equality
  - The tree of content elements can be traversed with code
  - Can be observed in hover tooltips or with [`repr`]
  - New [methods]($content) on content: `func`, `has`, `at`, and `location`
  - All optional fields on elements are now settable
  - More uniform field names (`heading.title` becomes `heading.body`,
    `list.items` becomes `list.children`, and a few more changes)

- Further improvements
  - Added [`figure`] function
  - Added [`numbering`]($math.equation.numbering) parameter on equation function
  - Added [`numbering`]($page.numbering) and
    [`number-align`]($page.number-align) parameters on page function
  - The page function's [`header`]($page.header) and [`footer`]($page.footer)
    parameters do not take functions anymore. If you want to customize them
    based on the page number, use the new [`numbering`]($page.numbering)
    parameter or [`counter`] function instead.
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
  - Fixed layout of [`terms`] list

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
    corresponding main font and the [`smallcaps`] function
- Improved default spacing for headings
- Added [`panic`] function
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
- [Box]($box) and [block] have gained `fill`, `stroke`, `radius`, and `inset`
  properties
- Blocks may now be explicitly sized, fixed-height blocks can still break across
  pages
- Blocks can now be configured to be [`breakable`]($block.breakable) or not
- [Numbering style]($enum.numbering) can now be configured for nested enums
- [Markers]($list.marker) can now be configured for nested lists
- The [`eval`] function now expects code instead of markup and returns an
  arbitrary value. Markup can still be evaluated by surrounding the string with
  brackets.
- PDFs generated by Typst now contain XMP metadata
- Link boxes are now disabled in PDF output
- Tables don't produce small empty cells before a pagebreak anymore
- Fixed raw block highlighting bug

## February 12, 2023 { #_ }
- Shapes, images, and transformations (move/rotate/scale/repeat) are now
  block-level. To integrate them into a paragraph, use a [`box`] as with other
  elements.
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
- Fixed relative displacement in [`place`] function
- Fixed that lines don't have a size
- Fixed bug where `{set document(..)}` complains about being after content
- Fixed parsing of `{not in}` operation
- Fixed hover tooltips in math
- Fixed bug where a heading show rule may not contain a pagebreak when an
  outline is present
- Added [`baseline`]($box.baseline) property on [`box`]
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
