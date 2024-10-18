---
title: Earlier
description: Changes in early, unversioned Typst
---

# Changes in early, unversioned Typst

## March 28, 2023
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

## March 21, 2023
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

## February 25, 2023
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

## February 15, 2023
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

## February 12, 2023
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

## February 2, 2023
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

## January 30, 2023
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
