#import "utils.typ": *

= Highlights <highlights>
- Typst now produces @guides:accessibility[_accessible_ PDFs] out of the box, with opt-in support for stricter checks and conformance to PDF/UA-1
- Typst now supports all @pdf:pdf-a[PDF/A standards]
- PDFs can now be used as @image.format[images] (thanks to #gh("LaurenzV"))
- Added support for @par.justification-limits[character-level justification] (can significantly improve the appearance of justified text)
- Added support for many more built-in elements in HTML export
- Added typed HTML API (e.g. @html.div) with individually typed attributes
- Added support for multiple @table.header[headers] and subheaders in tables
- Added @title element for displaying the document title
- Added @math.frac.style[`frac.style`] property for producing skewed and inline fractions

= PDF export <pdf-export>
PDF export was fully rewritten to use the new #link("https://github.com/LaurenzV/krilla")[`krilla`] library, fixing various bugs and enabling many improvements. Known fixes are listed below, but there will likely be other changes in how the output behaves. If you spot any regressions, please #link("https://github.com/typst/typst/issues")[report them on GitHub]. _(Thanks to #gh("LaurenzV") for creating krilla!)_ #pr(5420)

- Typst now produces _accessible_ PDFs out of the box. Such documents are suitable for consumption in a wide range of circumstances. That not only includes consumption by people with permanent or temporary disabilities, but also by those with different devices or preferences.
  - Typst PDFs are now _tagged_ by default. _Tags_ are rich metadata that PDF viewers can use to make the document consumable in other ways than visually (e.g., through a screen reader).
  - In addition, Typst can now emit documents conforming to the PDF/UA-1 standard. (PDF/UA-2 is not yet supported, but planned.)
  - There is an increasing amount of existing and upcoming legislation requiring documents to be accessible, for instance, the European Accessibility Act and the Americans with Disabilities Act.
  - For more details on all of this, read the new @guides:accessibility[Accessibility Guide].
- Typst now supports all PDF/A standards: PDF/A-1b, PDF/A-1a, PDF/A-2b, PDF/A-2u, PDF/A-2a, PDF/A-3b, PDF/A-3u, PDF/A-3a, PDF/A-4, PDF/A-4f, and PDF/A-4e. See the @pdf:pdf-a[expanded PDF/A documentation] for guidance on how to select a suitable standard. #pr(5420) #pr(7038)
- Typst now supports the PDF versions 1.4, 1.5, 1.6, and 2.0 in addition to PDF 1.7. See the @pdf:pdf-versions[relevant section of the PDF documentation] for details. #pr(5420)
- Added @pdf.artifact function for marking content as not semantically meaningful #pr(6619)
- Added experimental @pdf.header-cell, @pdf.data-cell, and @pdf.table-summary functions for enhancing accessibility of documents with complex tables. These functions are guarded by the `a11y-extras` feature. They do not have a final interface and will be removed in the future, either through integration into table functions or through full removal. #pr(6619)
- PDF heading bookmarks now contain the heading's numbering #pr(6622)
- @pdf.attach[Attachments]
  - Renamed `pdf.embed` to @pdf.attach (the old name will remain as a deprecated alias until Typst 0.15) #pr(6705)
  - The @pdf.attach.mime-type[`mime`] property of `pdf.attach` is now checked for syntactical correctness *(Minor breaking change)*
  - Fixed parsing of the @pdf.attach.data[`data`] argument of `pdf.attach` #pr(6435)
  - Attachments now smartly determine whether they should be compressed #pr(6256)
- Text extraction (i.e. copy paste)
  - Now works correctly even when multiple different characters result in the same glyph #pr(5420)
  - Spaces between words at which a natural line break occurred are now correctly retained for text extraction #pr(6866)
  - Fixed mapping of hyphenation artifacts to Unicode text #pr(6799)
- Images
  - CMYK images now work properly in PDF export #pr(5420)
  - Improved export of text in SVG images with a filter #pr(5420)
  - Improved compatibility of SVG images with Quartz rendering engine (the engine used in Apple Preview) #pr(5420)
  - Improved handling of SVG images with high group nesting depth #pr(5420)
- Fixed a bug with text in patterns #pr(5420)
- Fixed gradients with transparency #pr(5420)

= HTML export <html-export>
- Added support for many more built-in elements (the @reference:model[_Model_ category] is now fully covered)
  - The @image element #pr(6578)
  - The @footnote and @footnote.entry element #pr(6917)
  - The @outline and @outline.entry element #pr(6606)
  - The @bibliography element #pr(6952)
  - The @smartquote element #pr(6710)
  - The @sub and @super elements #pr(6422)
  - The @underline, @overline, @strike, and @highlight elements #pr(6510)
  - The @smallcaps element #pr(6600)
  - The @lower and @upper functions #pr(6585)
- Added typed HTML API (e.g. @html.div) with individually typed attributes #pr(6476)
  - For example, to generate a `video` element you can now write `[#html.video(width: 400, src: "sunrise.mp4")]` instead of `[#html.elem("video", attrs: (width: "400", src: "sunrise.mp4"))]`. Note how the `width` attribute takes an integer instead of a string.
- Added support for intra-doc @link targets #pr(6602)
- The @raw element
  - Added syntax highlighting support #pr(6691)
  - Block-level `raw` elements now emit both a `<code>` and a `<pre>` tag #pr(6701)
  - The @raw.lang[language tag] of `raw` elements is now preserved as a `data-lang` attribute on the `<code>` tag #pr(6702)
- The @document.author[`authors`] and @document.keywords[`keywords`] properties of the `document` function now yield corresponding HTML `<meta>` tags #pr(6134)
- The @html.elem function now supports custom HTML element names #pr(6676)
- Improved encoding of @html.frame #pr(6605)
- Empty attributes are now encoded with shorthand syntax (e.g. ```html <div hidden></div>```) #pr(6479)
- Zero-sized horizontal weak spacing (`{h(0pt, weak: true)}`) does not cause a "was ignored during HTML export" warning anymore, so it can be used to destruct surrounding spaces without producing any output, as in paged export #pr(6917)
- Fixed encoding of `<pre>` and `<textarea>` elements that start with a newline #pr(6487) #pr(6497)
- Fixed encoding of #link("https://html.spec.whatwg.org/#raw-text-elements")[raw text elements] #pr(6487) #pr(6720)
- Fixed sizing of @html.frame #pr(6505)
- Fixed @measure in HTML export #pr(7186)
- Fixed nested @html.frame[`html.frame`s] #pr(6509)
- Fixed that a @box without a body was ignored in HTML export #pr(6709)
- Fixed encoding of whitespace in HTML #pr(6750)

= SVG export <svg-export>
- Added support for COLR-flavored color glyphs #pr(6693)
- Reduced amount of `<g>` grouping elements that are generated #pr(6247)

= PNG export <png-export>
- Fixed crash when @text.size[text size] is negative #pr(7004)

= Visualize <visualize>
- Added support for using PDFs as @image[images] using the new #link("https://github.com/LaurenzV/hayro")[`hayro`] library. PDFs will be embedded directly in PDF export, rasterized in PNG export, and turned into SVGs in SVG and HTML export. _(Thanks to #gh("LaurenzV") for creating hayro!)_ #pr(6623)
- Added support for WebP images #pr(6311)
- Various minor improvements for SVG images (see the #link("https://github.com/linebender/resvg/blob/v0.45.1/CHANGELOG.md#0450---2025-02-26")[resvg 0.44 and 0.45 changelogs])
- SVG images can now refer to external image files #pr(6794)
- Clip paths are now properly anti-aliased #pr(6570)
- Fixed gradients on curves where the last segment is @curve.line #pr(6647)
- Fixed stroke cap handling of shapes with partial strokes #pr(5688)
- Fixed corner radius handling of shapes with partial strokes #pr(6976)
- Fixed crash when sampling across two coinciding gradient stops #pr(6166)

= Layout <layout>
- Added opt-in support for character-level justification in addition to word-level justification, configured via the new @par.justification-limits property. This is an impactful microtypographical technique that can significantly improve the appearance of justified text. #pr(6161)
- Fixed wrong linebreak opportunities related to object replacement characters #pr(6251)
- Fixed an issue where a breakable block would still produce an empty segment even if nothing fit into the first segment, leading to various undesirable behaviors in combination with fills, strokes, and @block.sticky[stickiness] #pr(6335)
- Fixed crash with set rule for column or rowspan on a grid cell #pr(6401)
- Fixed @text.cjk-latin-spacing[CJK-Latin-spacing] at manual line breaks #pr(6700) and at sub- and superscript boundaries #pr(7175)

= Math <math>
- Added @math.frac.style[`frac.style`] property with new options for skewed and inline fractions #pr(6672)
- Added @math.equation.alt property for setting an alternative description for an equation #pr(6619)
- Text handling
  - A single equation can now use multiple fonts #pr(6365)
  - Glyph layout in math now uses proper text shaping, leading to better handling of more complex Unicode features #pr(6336)
  - Generated characters in an equation (e.g. the `√` produced by `sqrt`) can now be targeted by text show rules #pr(6365)
  - Added @math.scr[`scr`] function for roundhand script font style #pr(6309)
  - Added `dotless` parameter to @math.accent[`accent`] (typically for rendering a dotless accented i or j) #pr(5939)
  - Script-style glyphs are now preferred at reduced math sizes #pr(6320)
  - Fixed @text.stroke in math #pr(6243)
  - Broken glyph assemblies are now prevented even when font data is incorrect #pr(6688)
- Layout
  - Fixed a bug with vertical accent positioning #pr(5941)
  - Fixed positioning of bottom accents #pr(6187)
  - Fixed a bug with layout of roots #pr(6021)
  - Improved layout of @math.vec[`vec`] and @math.cases[`cases`], making it consistent with @math.mat[`mat`] #pr(5934)
  - Removed linebreak opportunity before closing bracket in inline math #pr(6216)
- An @math.mat.augment[`augment`] line can now exist at the start and end of a matrix, not only in between columns and rows #pr(5806)
- Shorthands and multi-character numbers do not bind more tightly than fractions anymore in cases like `[$x>=(y)/z$]` #pr(5925) #pr(5996) *(Minor breaking change)*
- Named arguments passed to symbols used as function now raise an error instead of being silently ignored #pr(6192) *(Minor breaking change)*
- The @math.mid[`mid`] element does not force the @math.class[`{"large"}` math class] upon its contents anymore and instead defaults to `{"relation"}` #pr(5980)
- Fixed error in math parsing when `..` isn't followed by anything #pr(7105)
- Fixed the default math class of ⅋, ⎰, ⟅, ⎱, ⟆, ⟇, and ، #pr(5949) #pr(6537)

_Thanks to #gh("mkorje") for his work on math!_

= Model <model>
- Tables
  - Added support for multiple @table.header[headers] and subheaders in tables #pr(6168)
  - Table headers now force a rowbreak, i.e. an incomplete row before a header will not be filled with cells after the header #pr(6687)
  - Fixed a bug where @table.header[headers] and footers could accidentally expand to contain non-header cells #pr(5919)
- Added @title element for displaying the document title #pr(5618)
- Added @figure.alt property for setting an alternative description for a figure #pr(6619)
- @link[Link] hit boxes for text are now vertically a bit larger to avoid issues with automatic link detection in PDF viewers #pr(6252)
- The @link function will now produce an error when passed an empty string as a URL #pr(7049) *(Minor breaking change)*
- The value of the @enum.item.number[`number`] argument of `enum.item` now takes `{auto}` instead of `{none}` for automatic numbering #pr(6609) *(Minor breaking change)*
- Improved spacing of nested tight lists #pr(6242)
- Fixed that `{quotes: false}` was ignored for @quote.block[inline-level quotes] #pr(5991)
- Fixed @heading.hanging-indent[hanging indent] for centered, numbered headings #pr(6839)
- Fixed @footnote.entry show rules breaking links from footnote to entry #pr(6912)
- Hebrew numbering does not add Geresh and Gershayim anymore #pr(6122)

= Bibliography <bibliography>
- Built-in styles
  - Updated styles to their latest upstream CSL versions #pr(350, repo: "typst/hayagriva")
  - Renamed `{"chicago-fullnotes"}` to `{"chicago-notes"}` (the old name remains as a deprecated alias) #pr(6920)
  - Renamed `{"modern-humanities-research-association"}` to `{"modern-humanities-research-association-notes"}` (the old name remains as a deprecated alias) #pr(6994)
  - Added support for locator/supplement in alphanumeric style #pr(307, repo: "typst/hayagriva")

- Hayagriva format
  - Added #link("https://github.com/typst/hayagriva/blob/v0.9.1/docs/file-format.md#chapter")[`chapter` field] corresponding to CSL `chapter-number` and BibLaTeX `chapter` #pr(383, repo: "typst/hayagriva")

- BibLaTeX format
  - Fixed parsing of alphanumeric page ranges #pr(86, repo: "typst/biblatex")
  - Added support for `%` comment syntax #pr(80, repo: "typst/biblatex")
  - Fixed parsing of space-separated single character commands #pr(71, repo: "typst/biblatex")
  - Added "primaryclass" alias for "eprintclass" field #pr(75, repo: "typst/biblatex")
  - Added support for BibLaTeX `language` field #pr(317, repo: "typst/hayagriva")
  - Improved translation of BibLaTeX fields to `genre` and `serial-number` #pr(296, repo: "typst/hayagriva") #pr(369, repo: "typst/hayagriva")

- CSL handling
  - The bibliography rendering now uses @strong, @emph, and @smallcaps to express CSL font styling instead of directly adjusting the @text style, making styling easier #pr(6984)
  - Added support for date seasons, which are displayed when the month is missing #pr(391, repo: "typst/hayagriva")
  - Terms for "AD" and "BC" are now correctly used from the chosen locale #pr(364, repo: "typst/hayagriva")
  - Fixed how subsequent citations with differing supplements translate into CSL `ibid` and `ibid-with-locator` positions #pr(6171)
  - Fixed handling of `ibid` and `ibid-with-locator` positions in styles #pr(301, repo: "typst/hayagriva")
  - Fixed the `location` conditional in CSL styles for citations with no locator #pr(399, repo: "typst/hayagriva")
  - Fixed accesses of the year suffix resulting in wrong CSL renders #pr(400, repo: "typst/hayagriva")
  - Fixed regression where page variables were no longer supported in styles' `<number>` elements #pr(289, repo: "typst/hayagriva")
  - Fixed sorting and formatting of name parts #pr(287, repo: "typst/hayagriva") #pr(313, repo: "typst/hayagriva")
  - Fixed year suffix collapsing #pr(367, repo: "typst/hayagriva")
  - Fixed delimiters in locale-specific date formatting #pr(385, repo: "typst/hayagriva")
  - Fixed rendering of date ordinals #pr(366, repo: "typst/hayagriva")
  - Fixed rendering and sorting of dates with BC years #pr(334, repo: "typst/hayagriva") #pr(368, repo: "typst/hayagriva")
  - Fixed sorting for empty sort values #pr(390, repo: "typst/hayagriva")

= Text <text>
- The @sub and @super functions now use the `subs` and `sups` OpenType font features instead of special Unicode characters for typographic scripts, fixing semantical, sizing, and positioning issues #pr(5777)
- The @raw element
  - Tweaked default syntax-highlighting color scheme of @raw text to make the colors more accessible #pr(6754)
  - JSON keys and string values now use different colors in the default `raw` syntax highlighting @raw.theme[theme] #pr(6873)
  - Fixed a crash when a `raw` @raw.syntaxes[syntax] contains an unescaped trailing backslash #pr(6883)
  - Fixed tab indentation in @raw text with CRLF line terminators #pr(6961)
- Translations
  - Added term translations (a term being the "Section" in "Section 1") for many new languages by importing pre-existing translations from LaTeX packages #pr(6852)
  - Added term translations for Indonesian #pr(6108), Latvian #pr(6348), Croatian #pr(6413), Lithuanian #pr(6587), French #pr(7010), French (Canada) #pr(7098), Galician #pr(7019), Dutch #pr(7026), Danish #pr(7031), Slovenian #pr(7032), Italian #pr(7050), Chinese #pr(7044), Spanish #pr(7051), and Irish (Gaeilge) #pr(7024)
  - Improved term translations for Czech #pr(6101), Swedish #pr(6519), Galician #pr(7019), Dutch #pr(7026), Chinese #pr(7044), Spanish #pr(7051), Irish (Gaeilge) #pr(7024), and German #pr(7082)
  - Improved smart quotes for French #pr(5976), Ukrainian #pr(6372), Russian #pr(6331), and Arabic #pr(6626)
- An empty font list is not allowed anymore in @text.font #pr(6049) *(Minor breaking change)*
- Added a warning when using a variable font as those are not currently supported #pr(6425)
- Fixed usage of the same font with different @text.font[coverage] settings #pr(6604)
- Fixed hyphens not showing up when hyphenating at specific positions (where invisible metadata exists) #pr(6807)
- Fixed styling of repeated hyphens in languages with hyphen repetition #pr(6798)
- Last resort font fallback does not consider default ignorable characters anymore during font selection #pr(6805)
- Updated New Computer Modern fonts to version 7.0.4 #pr(6376)
- Updated data and shaper to Unicode 16.0.0 #pr(5407)

= Scripting <scripting>
- The `{in}` operator can now be used to check whether a definition is present in a module #pr(6498)
- Added `default` parameter to @array.first, @array.last #pr(5970), @array.join #pr(6932), @str.first, and @str.last #pr(6554) methods
- Added `by` parameter to @array.sorted for sorting with a comparison function #pr(5627)
- Added @str.normalize function for Unicode normalization #pr(5631)
- Added @direction.from[`from`], @direction.to[`to`], and @direction.sign[`sign`] methods to `direction` type #pr(5893)
- @label[Labels] cannot be empty anymore #pr(6332) *(Minor breaking change)*
- The WebAssembly runtime used by the @plugin system was updated and now supports SIMD #pr(6997)
- Numberings and counters now use 64-bit numbers instead of platform-dependent numbers for consistency across platforms #pr(6026)
- Improved consistency of how large numbers are handled in data loading functions #pr(6836)
- The @toml function is now guaranteed to return a @dictionary[dictionary] and @toml.encode must receive a dictionary (it already errored before when passed something else, but the new function signature makes the error clearer) #pr(6743)
- Serialization of @bytes in human-readable formats now uses @repr #pr(6743)
- Fixed slicing of last N elements of an array using the @array.slice.count[`count`] parameter #pr(6838)
- Fixed crash when the expression wrapped in a `{context}` expression is an anonymous function #pr(6975)
- Fixed equality check between @raw.line elements #pr(6560)
- Fixed @repr of @label[labels] being potentially syntactically invalid #pr(6456)
- Fixed @repr of functions modified through @function.with[`with`] #pr(6773)

= Introspection <introspection>
- The following elements are newly locatable (i.e. they can be discovered with @query, @locate, etc. without having a label): `par`, `table`, `enum`, `list`, `terms`, `title`, `figure.caption`, `footnote.entry`, `outline.entry`, `image`, `emph`, `strong`, `link`, `cite`, `raw`, `underline`, `overline`, `strike`, and `highlight` #pr(6619)
- Fixed issues with logical order in bidirectional text #pr(5887) #pr(6796)
- Fixed logical order of cells in right-to-left @grid #pr(6232)
- Fixed logical order of elements in @grid cells that span multiple pages #pr(7198)
- Fixed logical order of metadata at the starts and ends of paragraphs #pr(6881) #pr(6909)
- Fixed introspection @location.position[positions] of inline elements at the very start of a paragraph (previously the Y position of an element at the very start would differ from one in the middle of the first line) #pr(6881)

= Styling <styling>
- Fixed rare infinite loop in show rule application #pr(6259)

= Performance <performance>
- Optimized incremental compilation with a new algorithm that, in particular, eliminates cases of very slow compilation with heavy context usage #pr(6683)

= Command Line Interface <command-line-interface>
- Added `typst info` subcommand for emitting build and environment information #pr(6761)
- Added `typst completions` subcommand for retrieving shell completions #pr(6568)
- Added `TYPST_IGNORE_SYSTEM_FONTS` environment variable #pr(6092)
- Added `--ignore-embedded-fonts` flag and `TYPST_IGNORE_EMBEDDED_FONTS` environment variable for disabling the use of fonts embedded into the Typst binary #pr(7037)
- Added `--no-pdf-tags` flag for disabling the automatic generation of accessibility tags. These are emitted when no particular standard like PDF/UA-1 is selected to provide a baseline of accessibility. #pr(6619)
- Added `--target` parameter to `typst query` #pr(6405)
- Added `--deps` and `--deps-format` parameters for emitting a list of files compilation depends on. Supports the three formats `json`, `zero`, and `make`. #pr(7022)
- Deprecated the `--make-deps` flag in favor of `--deps` with `--deps-format make` #pr(7022)
- On Linux, the font search will now fall back to known font directories if none were loaded via Fontconfig. #pr(71, repo: "RazrFalcon/fontdb")
- The CLI will now warn when trying to watch the standard input #pr(6381)
- Fixed race condition when two separate Typst CLI processes concurrently try to install a package #pr(5979)
- Fixed incremental SVG export not writing output SVGs on changes that only affect the page (e.g. changing `page.fill`) #pr(6810)
- Fixed a rare potential crash when stack space couldn't be grown as expected #pr(6969)

= Tooling and Diagnostics <tooling-and-diagnostics>
- Errors in many kinds of external text files (e.g., bibliographies, JSON files, etc.) are now annotated within these files instead of at the positions where the files are loaded from a Typst file #pr(6308)
- Warnings originating from within @eval are now correctly emitted #pr(6100)
- Diagnostic messages and hints
  - Improved error messages related to parsing of numbers #pr(5969)
  - The error message for an unsuitable CSL style now mentions the name of the style #pr(6306)
  - Added hints to various deprecated items with the removal timeline #pr(6617)
  - Added hint for the error message when an X/Y pair is expected #pr(6476)
  - Added hint for a label that appears in both the document and the bibliography #pr(6457)
  - Added additional hint for show rule recursion depth error #pr(5856)
  - Fixed inconsistent formatting of code points and strings in error messages #pr(6487)
- Autocompletion
  - Labels will now be deduplicated in completions #pr(6516)
  - Math font autocompletions are now based on the presence of an OpenType MATH table instead of the word "Math" in the name #pr(6316)
  - Autocompletion immediately after a comma in a parameter list is now supported for explicitly triggered completions (e.g. via Ctrl/Cmd+Space) #pr(6550)
  - Citation style aliases are now displayed as autocompletions #pr(6696)
  - Fixed autocompletion false positives with cursor after parameter list #pr(6475)
  - Fixed autocompletion after partial identifier in math #pr(6415)
  - Fixed which definitions are suggested in math #pr(6415)
  - Fixed inapplicable method autocompletions being listed #pr(5824)
- Tooltips
  - Fixed tooltip for scoped functions (e.g. `calc.round`) #pr(6234)
  - Fixed tooltip and details for figure references #pr(6580)
  - Expression tooltips now use `×` instead of `x` to indicate a repeated value #pr(6163)
- Fixed jump from click (jumping to the source panel with a click in the preview) in presence of transformations and clipping #pr(6037)

= Symbols <symbols>
- Added many new symbols and variants; many more than could be listed here. View #link("https://github.com/typst/codex/blob/v0.2.0/CHANGELOG.md#version-020-october-7-2025")[the dedicated changelog] for a full listing.
- Code points that have a symbol and emoji presentation now have the correct variation selector attached depending on whether they appear in `sym` or `emoji`. That said, they still don't render consistently in Typst due to how font fallback works. #pr(114, repo: "typst/codex")
- The @symbol function can now be used to create symbols that comprise not just one character, but one full grapheme cluster #pr(6489)

= Deprecations <deprecations>
- The name `pdf.embed` in favor of the new name @pdf.attach #pr(6705)
- The `{"chicago-fullnotes"}` bibliography style in favor of `{"chicago-notes"}` #pr(6920)
- The `{"modern-humanities-research-association"}` bibliography style in favor of `{"modern-humanities-research-association-notes"}` #pr(6994)
- The `--make-deps` CLI flag in favor of `--deps` with `--deps-format make` #pr(7022)
- Various symbols, see the #link("https://github.com/typst/codex/blob/v0.2.0/CHANGELOG.md#deprecated")[deprecation section in the dedicated changelog] for a full listing

= Removals <removals>
- Removed compatibility behavior of type/str comparisons (e.g. `{int == "integer"}`), which was temporarily introduced in Typst 0.8 and deprecated in Typst 0.13 *(Breaking change)*

= Development <development>
- The `Default` impl for `Library` had to be removed for crate splitting and trait coherence reasons, but you can get a drop-in replacement via `use typst::LibraryExt` #pr(6576)
- The `PdfOptions` struct has a new `tagged` field, which defaults to `{true}` #pr(6619) #pr(7046)
- Fixed a potential panic in `World::font` implementations. Downstream `World` implementations might need to apply #link("https://github.com/typst/typst/pull/6117")[the same fix]. #pr(6117)
- Increased minimum supported Rust version to 1.88 #pr(6637)
- The Docker container now has the optional non-root user `typst` #pr(7058)
