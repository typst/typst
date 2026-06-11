#import "utils.typ": *

#let changelogs = (
  codex: "https://github.com/typst/codex/blob/v0.3.0/CHANGELOG.md",
  hayagriva: "https://github.com/typst/hayagriva/blob/v0.10.0/CHANGELOG.md",
  resvg: "https://github.com/linebender/resvg/blob/v0.47.0/CHANGELOG.md",
  hayro: "https://github.com/LaurenzV/hayro/compare/d0b540fc9ab8e18b4a7a000d1404139af8e9d023...34834627c0b4afa9c83c9b64d4d978b127030c77",
)

This section documents all changes to the Typst language and compiler between Typst 0.14.2 and 0.15.0. If you are migrating an existing document to Typst 0.15, make sure to check out the @changelog:0.15.0:migration-guide. It walks you through changes you may need to make to your existing documents to ensure compatibility with Typst 0.15.

= Highlights <highlights>
- Typst now supports @text.variations[variable fonts]
- HTML export now supports equations out of the box via MathML
- With the new, experimental @reference:bundle[_bundle_] export target, a single Typst project can output multiple files (e.g. a multi-page website)
- A single document can now contain multiple @bibliography.target[bibliographies]
- Typst can now target multiple @pdf:pdf-standards[PDF standards] at once
- The new @selector.within[`within`] selector simplifies many introspection use cases
- The new @divider element represents a thematic break that templates can style
- @color.spot[Spot colors] enable use of custom pigments in offset printing
- With the new file @path type, project-relative paths can be passed to packages
- The new, more general `typst eval` CLI subcommand supersedes `typst query`
- Layout @reference:context:compiler-iterations[convergence] issues now result in detailed diagnostics
- Two long-standing list layout issues with marker alignment and centering were fixed
- Paragraph handling in HTML export is improved, preventing unexpected paragraphs from appearing
- This documentation now has a print version

= Language <language>

== Syntax <syntax>
- @path[File paths] (e.g. in imports or @image function calls) may not contain backslashes anymore; instead forward slashes must be used #pr(7688) #breaking
- Added hints for invalid characters in code mode #pr(7752)
- Added hint when trying to use a unary operator directly in an embedded expression using a hash (e.g. `[#-30deg]`) #pr(8069)
- Fixed potential stack overflow crashes by enforcing a maximum parsing depth #pr(7207)
- Fixed incremental parsing of unclosed strings #pr(8067)

== Styling <styling>
- Text show rules now have tracebacks that include the matched text #pr(8088)
- Fixed a crash with text show rules that match on multi-character symbols #pr(8011)

== Scripting <scripting>
- Extended hint when built-in definitions are shadowed to set and show rules #pr(7131)
- Added hint when trying to spread one or multiple dictionaries into an array #pr(7798)
- Improved diagnostics for invalid method calls #pr(7865)
- Improved hint for unknown variables in math that are available in `std` #pr(7810)
- Fixed a misleading error message when trying to assign to a temporary return value #pr(8212)

= Library <library>

== Foundations <foundations>
- Added file @path type that is now accepted in all places where paths were previously only represented as strings #pr(7555)
  - A path constructed in one file can be used in another file, but will be resolved relative to its original file
  - Likewise, paths can be passed across package boundaries
  - The initial path type is very minimal, but additional features like file existence checks or directory walking are planned
- Collections
  - Added `map` and `filter` functions on @dictionary.filter[dictionaries] and @arguments.filter[arguments] #pr(7284)
  - Named arguments on @arguments values are now accessible with field access syntax #pr(8407)
  - The `slice` functions on @str.slice[strings] and @array.slice[arrays] will now error if passing both an `end` and a `count` #pr(7238) #minor-breaking
  - Added @array.range.inclusive[`inclusive` parameter] to `range` function #pr(8345)
- Calculation
  - Added @calc.asinh[`asinh`], @calc.acosh[`acosh`], @calc.atanh[`atanh`], and @calc.erf[`erf`] functions to `calc` module #pr(6967) #pr(7655)
  - Added `int.min` and `int.max` constants for the minimum and maximum representable integer, respectively #pr(8445)
  - Fixed behavior of @calc.quo[`quo`] for negative integers #pr(7392)
  - Fixed potential overflows in @calc.norm[`norm`], @calc.abs[`abs`], @calc.gcd[`gcd`], and @calc.lcm[`lcm`] #pr(8156) #pr(8352)
  - Floating-point calculations are now consistently deterministic across platforms #pr(7712)
- Date & time handling
  - The @datetime.today.offset[`offset` parameter] of `datetime.today` now accepts @duration[durations] as an alternative to integers, allowing for sub-hour precision offsets #pr(7691)
  - Addition and subtraction of @datetime[datetimes] and @duration[durations] now retains precision instead of clamping to full days in some cases #pr(7843)
  - The @datetime constructor now emits more precise errors when components are missing #pr(7371)
- Conversions
  - Added @int.constructor.base[`base` parameter] to `int` constructor to configure in which base to parse a string #pr(7386)
  - Fixed that the @str.constructor.base[`base` parameter] of the `str` constructor was accepted for non-integer values if its value was `{10}` #pr(7237) #minor-breaking
  - Added hint when trying to @str.constructor[construct a string] with base 1 #pr(7237)
- The @panic function now displays strings as-is instead of showing their @repr, making it more suitable for friendly, user-facing messages #pr(8211)
- Changed @repr of styles and @location[locations] to be more distinct #pr(7364)

== Model <model>
- Added @divider element representing a thematic break that templates can style #pr(7982)
- Bundle-related elements
  - The @document element can now be constructed to produce individual documents in bundle export #pr(7964)
  - Added @document.path[`path`] and @document.format[`format`] parameters to `document` element #pr(7964)
  - Added experimental @asset element #pr(7964)
- Bibliography management
  - A single document can now contain multiple bibliographies #pr(8307)
  - Added @bibliography.target[`target` parameter] to `bibliography` element to configure which citation is picked up by which bibliography #pr(8307)
  - Added @bibliography.group[`group` parameter] to `bibliography` element to configure how numbers are shared/reset across bibliographies #pr(8307)
  - Added support for numeric values for the `month` key in `.bib` files #pr(104, repo: "typst/biblatex")
  - Added support for BibLaTeX name options in `.bib` files #pr(90, repo: "typst/biblatex")
  - Added support for propagating non-numeric `volume` fields in `.bib` files to bibliographies #pr(478, repo: "typst/hayagriva")
  - Improved sorting in bibliographies to take into account language conventions #pr(314, repo: "typst/hayagriva")
  - Improved interoperability with CSL styles; for a full listing of changes, review #link(changelogs.hayagriva)[the Hayagriva 0.10.0 changelog]
  - Added support for setting directors on videos without a parent in Hayagriva YAML files #pr(450, repo: "typst/hayagriva")
  - Improved handling of `Anthos` entries in Hayagriva YAML files by treating them as `chapter`s in CSL #pr(427, repo: "typst/hayagriva")
- Footnotes
  - The link of a @footnote is now within the superscript instead of around it, improving PDF tagging and HTML output #pr(7637)
  - The thickness of the default @footnote.entry.separator[footnote separator] is now specified in font-relative instead of absolute units #pr(7950)
- Numbering
  - Added @numbering support for Armenian numerals #pr(7529), Arabic Abjad numerals #pr(170, repo: "typst/codex"), and Tibetan numerals #pr(163, repo: "typst/codex")
  - Greek @numbering now uses the modern Greek style #pr(7632)
- The @par.first-line-indent property will now _fold,_ meaning that partial dictionaries across different set rules or @par calls are combined #pr(7747)
- Added @list.marker-align property for defining how to align list markers #pr(7895)
  - When omitted, it will default to the new baseline alignment (vertically), combined with `end` alignment (horizontally)

== Text <text>
- Added support for variable fonts #pr(8425)
  - The well-known variation axes `ital`, `slnt`, `wght`, `wdth`, and `opsz` are automatically set based on text @text.weight[`weight`], @text.stretch[`stretch`], @text.style[`style`], and @text.size[`size`]
  - Custom variations can be configured via the new @text.variations[`variations` parameter] of the `text` function
  - When using a variable font with Typst, the suffixes "Variable", "Var", and "VF" should be omitted as Typst trims them to unify static and variable fonts into a single family #pr(8444) #minor-breaking
- Font features
  - The @text.alternates parameter now accepts an integer in addition to a boolean to select stylistic alternates other than `{0}` and `{1}` #pr(7423)
  - Parsing of tag names in @text.features is now more strict #pr(7423) #minor-breaking
- Fixed that `{context text.font}` did not reflect the `covers` field #pr(7354)
- Fixed uneven @text.cjk-latin-spacing[CJK-Latin spacing] in justified paragraphs #pr(7606)
- Fixed a bug where the @lorem function would not produce the exact number of requested words #pr(7877)
- Improved translations for Swedish #pr(7166), Portuguese #pr(7088), Czech #pr(7318), Latvian #pr(7701), Slovak #pr(7734), Polish #pr(7734), Vietnamese #pr(7774), Finnish #pr(7988) #pr(7989), and Welsh #pr(7811)
- Added font exception to avoid _SimSun-ExtB_ being incorrectly merged with _SimSun_ #pr(8042)
- Updated New Computer Modern fonts to version 8.1.0 #pr(7597) #pr(7663) #pr(8164) #pr(8330) #pr(8435)
- Updated Unicode components #pr(8406)
  - In particular, this fixed an issue with linebreaking of guillemets

== Math <math>
- Layout
  - Improved layout of under/over elements like @math.underbrace[`underbrace`] #pr(7299)
  - Slightly improved spacing around @math.op elements #pr(7429)
  - In which situations _cramped_ styles (with tighter spacing) are applied is now fully consistent with TeX and MathML Core #pr(8082)
  - The @math.lr.size[`size` parameter] of the `lr` function now consistently applies to @math.mid[middle delimiters] in the same way it does to outer delimiters #pr(7435)
  - The @math.lr.size[`size` parameter] of the `lr` function now resolves relatively to the height of just the inner content; it does not take the delimiters into account anymore #pr(7605) #minor-breaking
  - Glyph @math.stretch[stretching] is now always relative to the base glyph rather than a potentially already scaled version (e.g. due to display sizing) #pr(7435) #minor-breaking
  - Fixed left/right alignment not being applied correctly due to spacing next to alignment points #pr(7473) #pr(7435)
  - The @math.binom[`binom`] element now uses different OpenType constants for layout; though this does not lead to visible changes with most fonts #pr(7685)
  - The default length and stroke width of @math.cancel lines is now specified in font-relative instead of absolute units #pr(7241)
  - Fixed potential misalignment in @math.cases[`cases`] function #pr(7662)
- Text handling
  - Improved handling of multi-character symbols in math #pr(6929)
  - Fixed that some glyphs did not stretch correctly in script sizes #pr(8171)
  - Fraction, root, and under/over lines now respect @text.stroke #pr(7540)
  - Accents in math are now always rendered in front of their base if they overlap #pr(7733)
- The @math.class[`class`] function now applies the class only to its direct body rather than recursively #pr(8328) #minor-breaking
- More delimiter symbols (e.g. `chevron.l`) are now callable to produce an @math.lr[`lr`] element #pr(7228) #minor-breaking
- Fixed various bugs with rendering of mathematical expressions that look like function calls but in reality aren't (e.g. `[$pi(1, 2)$]`, since `pi` is not a function) #pr(7865)
- Fixed a bug with ordering of primes and nested attachments #pr(7647)

== Symbols <symbols>
- Added many new symbols and variants. View #link(changelogs.codex)[the `codex` 0.3.0 changelog] for a full listing.

== Layout <layout>
- Baseline information is now retained in many more parts of the layout engine #pr(8150) #breaking
  - In particular, text contained in a @box with an inset is now aligned with the text surrounding the `box`
  - This also fixes a bug where wrapping an inline equation in a `box` would shift its baseline
  - Similarly, using a @block in an equation will keep the baseline intact
  - Last but not least, the marker/number and item of a @list or @enum are now properly baseline-aligned with the first line of the item even if the item is vertically larger than a normal line #pr(7895)
- Centering something in a list now centers based on the full available width rather than based on the maximum width of other list content #pr(7895)
- Page layout
  - Added @page.bleed[`bleed` parameter] to `page` element to set up bleed margins #pr(6357)
  - Fixed the size of `{"us-executive"}` @page.paper[paper] #pr(7869)
  - Added warning for `{show page}` rules as they are unsupported #pr(7445)
- Paragraph layout
  - Fixed a bug where justified text could accidentally protrude into the margin when it ends with certain kinds of characters (e.g. a zero-width space) #pr(8415)
  - Fixed a bug where @par.first-line-indent[first-line indent] was applied at the start of a column even if `{all: false}` is set #pr(7722)
- Added support for spacing that is both @v.weak[weak] and @fraction[fractional] #pr(6833)

== Visualize <visualize>
- Added support for @color.spot[spot colors] (also called separation colors) #pr(7629)
- Tilings
  - Added @tiling.constructor.offset[`offset` parameter] for shifting the starting position of a tiling #pr(7506)
  - Fixed parent-relative placement for @stack and @polygon #pr(8324)
- Gradients
  - Added @color:predefined-color-maps[`color.map.coolwarm`] for use with gradients #pr(7489)
  - Fixed interpolation of gradient @gradient.stops[stops] in Oklab color space #pr(7326)
  - Fixed gradient @gradient.angle[angle] handling for negative-size shapes #pr(7826)
  - Fixed gradient strokes for @line[lines] and @curve[curves] #pr(7863)
  - Fixed parent-relative placement for @stack and @polygon #pr(8324)
- Fixed various bugs with rectangle strokes in combination with @rect.radius[radii] #pr(7357) #pr(8081)
- Fixed a potential deadlock with font fallback in SVGs #pr(7766)
- Various improvements to SVG image handling (see the #link(changelogs.resvg)[resvg 0.46 and 0.47 changelogs]); in particular:
  - Added support for SVGs without top-level `xmlns` attribute
  - Added support for variable fonts in SVG using the `font-variation-settings` CSS property
- Various improvements to PDF image handling (see the #link(changelogs.hayro)[commits between `d0b540f` and `3483462` on hayro]); in particular:
  - Added support for JPEG2000 (`JPXDecode`) and JBIG2 (`JBIG2Decode`) images
  - Improved parsing robustness for non-compliant files
  - Added support for blend modes

== Introspection <introspection>
- Layout convergence issues now result in detailed diagnostics that help pin down the cause #pr(7364)
- Added @selector.within[`within` selector] that matches elements that are contained within any elements matching an ancestor selector #pr(8250) #pr(7964) #pr(8114)
- Added @counter.display.at[`at`] parameter to `counter.display` function #pr(6781)
- Improved how @counter.display auto-selects the numbering to use #pr(7446)

== Data Loading <data-loading>
- Added support for namespaces to @xml function #pr(7899)
- Added hint when trying to read from a path that looks like a URL #pr(7682)
- Diagnostics for binary file loading failures now include file paths #pr(8259)
- The @json function now emits a friendly error when the loaded JSON has a leading UTF‑8 BOM #pr(7488)

= Export <export>

== Bundle <bundle-export>
- Added new, experimental @reference:bundle[_bundle_ export target] #pr(7964)
  - With bundle export, you can emit multiple output files from a single Typst project
  - Bundles can contain any combination of @html[HTML pages], @pdf[PDFs], @reference:png[PNGs], @reference:svg[SVGs], and arbitrary @asset[assets]

== HTML <html-export>
- Mathematical equations are now automatically exported to MathML _(thanks to #gh("mkorje"))_ #pr(7436)
  - MathML defines how to render an equation, but also preserves its semantics
  - If you've previously relied on show rules that use @html.frame to render equations to SVG, try MathML output. Switching to it will improve the accessibility of your document (though rendering will be somewhat less consistent across browsers).
- The @box and @block elements' purpose is now aligned with paged export #pr(8181) #breaking
  - @box is used to bring block-level content inline
  - @block ensures inline-level content becomes block-level
  - Depending on the contained content, this may be achieved by setting the CSS `display` property or by wrapping in an additional `<span>` or `<div>`.
- The rules of how paragraphs are grouped in HTML have been adjusted to fix cases where paragraphs would appear unexpectedly #pr(8181) #pr(7505) #breaking
  - The list of HTML elements that can be grouped into paragraphs was tweaked (it now includes all _phrasing content_ with the exception of elements that default to `display: none`)
  - This default can be controlled by wrapping an element in a @box or @block as appropriate
  - HTML elements that cannot be part of paragraphs (like a `<div>`) do not immediately force adjacent inline-level Typst content to be wrapped in a paragraph; rather, they are considered _neutral_ for paragraph grouping
  - Paragraph creation is only forced by _block-level Typst elements_ (as opposed to HTML elements). Built-in block-level elements like headings or images wrap the HTML elements they create in @block elements to force adjacent inline content into paragraphs. Package authors should do the same to ensure paragraph creation is consistent between HTML and paged export.
- The @target function can now be used without the `html` feature flag (the rest of HTML export remains feature-flagged) #pr(8248)
- DOM structure and built-in show rules
  - The root `<html>` element now receives a `lang` attribute respecting what was configured for @text.lang #pr(7208)
  - The Typst @image element now always emits `width` and `height` attributes on the generated HTML `<img>` element #pr(8118)
  - Code in `<pre>` tags will now prefer raw newlines over `<br>` elements to encode line breaks #pr(7675)
  - Fixed generated HTML for @quote.attribution[quote attributions] #pr(8181)
  - Fixed table cell show rules not working in HTML export #pr(7821)
- Serialization
  - HTML is now minified by default; use the `--pretty` CLI flag or the checkbox in the web app to pretty-print it #pr(8371)
- HTML elements
  - The @html.elem.attrs[`attrs` parameter] on `html.elem` will now _fold,_ meaning that partial attributes across different set rules or `html.elem` calls are combined #pr(8182)
  - @html.script and @html.style only accept a string and not arbitrary content anymore #pr(7784) #breaking
- Whitespace handling
  - HTML `<br>` elements now collapse adjacent Typst spaces #pr(8166)
  - Fixed spans being emitted to protect whitespace from collapsing unnecessarily in some cases #pr(8166)

== PDF <pdf-export>
- Typst can now target multiple (compatible) PDF standards at once, e.g. PDF/UA-1 and PDF/A-2a #pr(8294)
- PDFs are now a bit more space-optimized at the cost of being harder to inspect with a text editor; use the `--pretty` CLI flag or the checkbox in the web app to pretty-print them #pr(8294) #pr(8371) #pr(8430)
- Labelled headings now result in named destinations even if they are not referenced #pr(7964)
- Graphics
  - Added support for more compositing features of COLRv1 fonts #pr(358, repo: "LaurenzV/krilla")
  - Fixed rendering of @gradient[gradients] in LinearRGB, CMYK, and Luma color spaces #pr(8149)
  - Fixed excessive sampling of linear gradients #pr(7818)
- Tagging
  - Added support for more specific artifact kinds in @pdf.artifact; these are now internally used when appropriate #pr(8416)
  - Fixed "invalid document structure" errors with complex @list.marker[list markers] #pr(7789)
  - Fixed wrong PDF tagging order for inline content outside of paragraphs #pr(7861)
  - Fixed bounding box computations for stroked shapes in tagged PDFs #pr(8322)
- Standards compliance
  - Fixed potentially incompliant PDF files by emitting an error when PDF/UA-1 is requested but complying with PDF/UA-1 would require newer features than available in the current PDF version #pr(278, repo: "LaurenzV/krilla")
  - Fixed potentially incompliant PDF 1.4 files by emitting errors when implementation limits are exceeded #pr(348, repo: "LaurenzV/krilla")

== SVG <svg-export>
- SVGs are now minified by default; use the `--pretty` CLI flag or the checkbox in the web app to pretty-print them #pr(8371)
- Somewhat reduced the size of generated SVGs #pr(7476) #pr(7680) #pr(7857)
- Graphics
  - Fixed a bug where @tiling[tilings] could be incorrectly reused in multiple places #pr(7837)
  - Fixed rendering of @gradient[gradients] in LinearRGB, CMYK, and Luma color spaces #pr(8149)
  - Fixed handling of @gradient.conic.angle[conic gradient angles] #pr(7952)
  - Fixed excessive sampling of linear gradients #pr(7818)
- Fixed positioning and sizing of color bitmap glyphs #pr(7679) #pr(8440)
- Fixed sources of non-determinism in SVG export #pr(7680)

== PNG <png-export>
- Fixed handling of @gradient.conic.angle[conic gradient angles] #pr(7952)
- Fixed positioning and sizing of color bitmap glyphs #pr(8440)
- Fixed that negatively @scale[scaled] text with equal `x` and `y` scale would turn invisible in PNG export #pr(8111)

= Command Line Interface <command-line-interface>
- Added new `typst eval` subcommand to evaluate a Typst code expression from the CLI; this command supersedes `typst query` #pr(7362)
- Tracebacks for diagnostics are now more compact and readable #pr(8000)
- Added `--pretty` flag for producing human-readable output; output is otherwise minified by default (applies to HTML, SVG, and PDF, but not PNG) #pr(8371)
- Fonts
  - The output of `typst fonts --variants` is now more readable and informative; in particular, it also displays the paths of font files and, for variable fonts, variation axes #pr(7490) #pr(8425)
  - The CLI now discovers fonts lazily, saving time on operations that do not need fonts (like HTML export without @html.frame[frames]) #pr(7380)
  - Adobe Creative Cloud fonts are now discovered as system fonts #pr(7716)
- Dependency output
  - When writing to stdout while using `--deps-format make` (which is incompatible and thus fails), no empty Make dependency file will be emitted anymore #pr(7246)
  - The JSON dependency format now includes information about outputs in addition to inputs #pr(7209)
- Non-Unicode input paths are not supported anymore #pr(7688) #breaking
- The experimental `--timings` argument now requires an explicit file name instead of defaulting to `record-{n}.json` #pr(8119) #breaking
- Added colors to `--help/-h` and `typst info` output #pr(7443) #pr(7500)
- Fixed inconsistency in environment variable handling between `typst info` and `typst compile` #pr(8030)
- Fixed a bug with the injection of a live reload script when using `typst watch` with HTML export #pr(7770)
- Fixed a bug where local timezone information was taken into account even if a fixed date is set via `--creation-timestamp` or `SOURCE_DATE_EPOCH`, leading to irreproducible results #pr(7856)

= Tooling <tooling>
- Syntax highlighting
  - In math, parentheses used for grouping are now highlighted differently than ones intended for display #pr(7894)
- Autocomplete and tooltips
  - Autocompletion and tooltips are now aware of parameters of user-defined functions #pr(7808)
  - Autocomplete descriptions and tooltips for font families are now more detailed and, for variable fonts, include variation axes #pr(8425)
  - Function autocompletions in math mode now always prefer round parentheses over square brackets (which are not supported in math) #pr(8417)
  - Fixed autocompletion and tooltips not working with argument lists in math mode #pr(8116)

= Deprecations <deprecations>
- Certain unclear/ambiguous ways to write a raw language tag; these will now emit a warning in anticipation of an @raw:language-tag-changes[upcoming change to how they are parsed] #pr(8257)
- Fallback to Arabic numerals for @numbering systems that do not support the number zero (e.g. for `{"⓵"}`) #pr(7936)
- Various symbols were renamed, see the #link(changelogs.codex + "#deprecations-in-sym")[deprecation section in the dedicated changelog] for a full listing
- Some citation styles that were renamed or superseded #pr(404, repo: "typst/hayagriva") #pr(424, repo: "typst/hayagriva") #pr(453, repo: "typst/hayagriva")
  - `council-of-science-editors` is now called `cse-citation-sequence-brackets-8th-edition`
  - `council-of-science-editors-author-date` is now called `cse-name-year`
  - `modern-language-association-8` / `mla-8` is superseded by `modern-language-association` / `mla`
  - `vancouver` is now called `nlm-citation-sequence`
  - `vancouver-superscript` is now called `nlm-citation-sequence-superscript`
- Undocumented array forms of @enum and @terms items #pr(7484)

= Removals <removals>
- The `path` element, use @curve instead #pr(7554) #breaking
- The `pattern` type, use @tiling instead #pr(8252) #breaking
- The `pdf.embed` element, use @pdf.attach instead #pr(8252) #breaking
- The scoped functions `cbor.decode`, `csv.decode`, `json.decode`, `toml.decode`, `xml.decode`, `yaml.decode`, and `image.decode`; directly pass @bytes to the top-level functions instead #pr(8252) #breaking
- Various previously deprecated symbols, see the #link(changelogs.codex + "#removals-in-sym-breaking-change")[removals section in the dedicated changelog] for a full listing #breaking

= Development <development>
- The `typst-kit` crate was completely reworked to make it easier to create a Typst `World` implementation #pr(7710) #pr(8026)
- Diagnostic hints can now have spans (though typically they will be _detached,_ which just means there isn't a span) #pr(7364)
- Increased minimum supported Rust version to 1.92 #pr(8236)
- Moved Nix flake from #repo("typst/typst") to #repo("typst/typst-flake"), where it is now maintained by the community as a best effort #pr(7512)

= Migration guide <migration-guide>
This section walks you through changes you may need to make to your existing documents to ensure compatibility with Typst 0.15.

Typst 0.15 @changelog:0.15.0:removals[removes] definitions from the Typst standard library that were already deprecated in previous Typst versions. If you are still relying on those, also refer to the #link("https://typst.app/blog/2025/typst-0.14#migrating")[Typst 0.14] and #link("https://typst.app/blog/2025/typst-0.13#migrating")[Typst 0.13] migration guides.

== Baselines <baselines>
Typst now retains baseline information in more parts of the layout engine (most importantly for @box[boxes], @block[blocks], @list.item[list items], and @math.equation[equations]). This may lead to silent layout shifts. These are generally improvements, but if you've previously counter-acted bad-looking output through explicit adjustments, you may need to revisit those.

In particular, Typst will now set the baseline of a @box to the baseline of its first line of content and use it to align the `box` with its surrounding text. This yields better-looking output out of the box.

```example
// Typst 0.15+
Lorem #box(inset: 0.3em, stroke: 1pt)[ipsum] dolor.

// Typst 0.14 and below
<<< Lorem #box(inset: 0.3em, stroke: 1pt)[ipsum] dolor.
>>> Lorem #box(inset: 0.3em, stroke: 1pt, baseline: -0.3em)[ipsum] dolor.
```

== File paths <file-paths>
If you are using Typst on Windows, you may have used backslashes in your file paths. This syntax is not supported anymore. Consistent use of forward slashes ensures your document compiles on all supported platforms.

```typ
// Typst 0.15+ ✅
#include "chapters/introduction.typ"

// Typst 0.14 and below ❌
#include "chapters\\introduction.typ"
```

Moreover, the Typst CLI does not support non-Unicode input paths anymore. Such paths are exceedingly rare today.

== #short-or-long[Math stretching][Math glyph stretching] <math-stretching>
In mathematical typesetting, glyphs frequently need to be stretched to the size of some part of an equation. This primarily applies to brackets, but also accents, arrows, and more. In Typst, this happens implicitly through matched delimiters or explicitly through the @math.lr[`lr`] and @math.stretch[`stretch`] functions.

Typst 0.15 makes some adjustments to how ratios passed to these functions are resolved. Previously, a size of `{200%}` passed to @math.lr.size[`lr.size`] or @math.stretch.size[`stretch.size`] would be interpreted relative to a glyph size that was potentially already affected by @math.display[display sizing]. Now, ratios are always resolved relative to the base glyph size. In practice, this means that, to retain the same appearance, you may need to increase the target sizes you've configured for glyphs that are already larger out of the box in display style (e.g. integrals).

== #short-or-long[Math delimiters][Math delimiters as functions] <math-delimiters>
In Typst's math mode, various symbols defined in @sym can be called like a function, either to be applied as an accent or to delimit body content. Typst 0.15 extends this handling to additional delimiters like `chevron.l`. This can change the output of existing calls, which would previously fall back to displaying the parentheses (which would rarely have produced desirable output in the case of delimiters).

```example
// Typst 0.15+ ✅
$ chevron.l(x) $

// Typst 0.14 and below ⚠️
<<< $ chevron.l(x) $
>>> $ chevron.l (x) $
```

== Math classes <math-classes>
The @math.class[`class`] function defines how part of an equation should be laid out (primarily the spacing around it). In Typst 0.14 and below, the `class` function applied recursively: If applied to a larger piece of content, all items in it would receive the class. In Typst 0.15, the class is only applied to the directly wrapped content. This is mostly a bug fix, but can lead to subtle layout changes.

== #short-or-long[HTML paragraphs][Paragraphs, boxes, and blocks in HTML] <html-paragraphs>
Typst automatically @par:what-becomes-a-paragraph[collects inline-level content into paragraphs]. In Typst 0.14 and below, the exact same rules were used to collect inline-level HTML elements (e.g. a `<span>`) into Typst paragraphs, which in turn result in `<p>` elements. This had the unfortunate effect that even use of the low-level @html:typed[typed HTML API] could result in `<p>` elements appearing automatically. Consider the example below:

```typ
#html.div({
  html.span[Hello]
  html.div[World]
})
```

Where Typst 0.14 would insert an unexpected additional `<p>` element, Typst 0.15 yields the expected result:

```html
<!-- Typst 0.14 and below ⚠️ -->
<div>
  <p><span>Hello</span></p>
  <div>World</div>
</div>

<!-- Typst 0.15+ ✅ -->
<div>
  <span>Hello</span>
  <div>World</div>
</div>
```

This change in output is achieved through a few new rules for how paragraphs are collected in HTML export. Previously, Typst would always categorize elements into either inline-level or block-level. It would then force inline-level elements into a paragraph whenever at least one block-level element was present in the same flow of content. In Typst 0.15, elements can instead be considered inline-level, block-level, or _neutral:_

- Text, Typst @box elements and HTML _phrasing content_ (elements that are allowed as children of `<p>` elements) are considered inline-level #footnote[There are a few exceptions: Some elements like `<script>` are considered phrasing content by the HTML specifications, but don't make sense in paragraphs as they default to `display: none`.]
- All other HTML elements are considered neutral
- Typst @block elements are considered block-level

Neutral elements don't force adjacent inline-level content into paragraphs. They can co-exist with inline-level content in a _mixed flow._ This ensures that usage of the typed HTML API does not result in extraneous paragraphs.

Meanwhile, Typst elements that are block-level out of the box (e.g. @heading) now explicitly use a @block in their default show rule to ensure that they still force adjacent inline-level elements into paragraphs. This ensures consistency in paragraph grouping across paged and HTML export. Package authors should also explicitly use @block[blocks] when creating HTML components that Typst should consider block-level.

To make this change possible, the way @box[boxes] and @block[blocks] behave in HTML export has been adjusted: Instead of unconditionally wrapping their contents in a `<span>` or `<div>`, they now do just what is necessary to ensure their contents are considered inline- or block-level by browsers. If they contain multiple children, they still create a wrapper element, but for just a single child, they instead configure the CSS `display` property. And if the single child already has the appropriate `display` property by default, it is fully omitted. Then, the effect of the `box` or `block` is limited to influencing Typst's paragraph grouping.

== #short-or-long[HTML raw text elements][HTML `script` and `style` elements] <html-script-and-style>
The typed HTML functions for the HTML @html.script[`script`] and @html.style[`style`] element previously accepted arbitrary body content and then only failed during HTML serialization if the content resulted in non-textual elements. In Typst 0.15, they instead only accept strings in the first place. Note that you can also use them with @raw syntax by accessing the `.text` field.

````typ
// Typst 0.15+ ✅
#html.style("a { color: red }")
#html.style(
  ```css
  a { color: red }
  ```.text
)

// Typst 0.14 ❌
#html.style[
  a { color: red }
]
````

== #short-or-long[Variable families][Variable font family names] <variable-families>
Typst aims to unify different fonts from the same family under a single family name. To that effect, it automatically trims common style suffixes like "Bold" or "Condensed" from font family names. Instead of selecting these through the name, they should be accessed through Typst's built-in mechanisms (such as the @text.weight[`weight`] and @text.stretch[`stretch`] parameters).

In Typst 0.15, the additional suffixes "Variable", "Var", and "VF" are trimmed to unify static and variable fonts into a single family. If you've previously used a font with any of these suffixes in its family name, you should now omit the suffix when specifying the @text.font[`font`].

== #short-or-long[Language tags][Upcoming raw language tag changes] <language-tags>
In the next Typst version, we will make a change to how @raw:language-tag-changes[raw language tags are parsed]. Should this upcoming change have an effect on your document, the compiler will already warn you today and suggest how to adjust your document to future-proof it against the change.

== #short-or-long[Numbering fallback][Numbering fallback for the number zero] <numbering-fallback>
Not all @numbering systems can express the number zero. In Typst 0.14 and below, those would silently fall back to Arabic numerals for `{0}`. This behavior is deprecated in Typst 0.15. Using zero with these systems will become a hard error in the future.

== #short-or-long[Validation][Tightened validation in some functions] <validation>
The @array.slice function, @str constructor, and @text.features parameter now perform stricter validation on their inputs. If you've previously passed invalid or nonsensical input, you will now receive an error.

== #short-or-long[Citation styles][Renamed citation styles] <citation-styles>
Typst 0.15 @changelog:0.15.0:deprecations[renames] some citation styles to stay aligned with upstream CSL changes. If you are relying on any of these, the compiler will warn you and suggest the new name.

== #short-or-long[Symbols][Renamed symbols] <citation-styles>
Typst 0.15 also #link(changelogs.codex + "#deprecations-in-sym")[renames] a few symbols in @sym. If you are using any of these, the compiler will warn you and suggest the new name.
