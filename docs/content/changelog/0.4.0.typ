#import "utils.typ": *

= Footnotes <footnotes>
- Implemented support for footnotes
- The @footnote function inserts a footnote
- The @footnote.entry function can be used to customize the footnote listing
- The `{"chicago-notes"}` @cite.style[citation style] is now available

= Documentation <documentation>
- Added a @guides:for-latex-users[Guide for LaTeX Users]
- Now shows default values for optional arguments
- Added richer outlines in "On this Page"
- Added initial support for search keywords: "Table of Contents" will now find the @outline[outline] function. Suggestions for more keywords are welcome!
- Fixed issue with search result ranking
- Fixed many more small issues

= Math <math>
- *Breaking change*: Alignment points (`&`) in equations now alternate between left and right alignment
- Added support for writing roots with Unicode: For example, `[$root(x+y)$]` can now also be written as `[$√(x+y)$]`
- Fixed uneven vertical @math.attach[`attachment`] alignment
- Fixed spacing on decorated elements (e.g., spacing around a @math.cancel[canceled] operator)
- Fixed styling for stretchable symbols
- Added `tack.r.double`, `tack.l.double`, `dotless.i` and `dotless.j` @sym[symbols]
- Fixed show rules on symbols (e.g. `{show sym.tack: set text(blue)}`)
- Fixed missing rename from `ast.op` to `ast` that should have been in the previous release

= Scripting <scripting>
- Added function scopes: A function can now hold related definitions in its own scope, similar to a module. The new @assert.eq function, for instance, is part of the @assert function's scope. Note that function scopes are currently only available for built-in functions.
- Added @assert.eq and @assert.ne functions for simpler equality and inequality assertions with more helpful error messages
- Exposed @list.item[list], @enum.item[enum], and @terms.item[term list] items in their respective functions' scope
- The `at` methods on @str.at[strings], @array.at[arrays], @dictionary.at[dictionaries], and @content.at[content] now support specifying a default value
- Added support for passing a function to @str.replace[`replace`] that is called with each match.
- Fixed @str.replace[replacement] strings: They are now inserted completely verbatim instead of supporting the previous (unintended) magic dollar syntax for capture groups
- Fixed bug with trailing placeholders in destructuring patterns
- Fixed bug with underscore in parameter destructuring
- Fixed crash with nested patterns and when hovering over an invalid pattern
- Better error messages when casting to an @int[integer] or @float[float] fails

= Text and Layout <text-and-layout>
- Implemented sophisticated CJK punctuation adjustment
- Disabled @text.overhang[overhang] for CJK punctuation
- Added basic translations for Traditional Chinese
- Fixed @raw.align[alignment] of text inside raw blocks (centering a raw block, e.g. through a figure, will now keep the text itself left-aligned)
- Added support for passing a array instead of a function to configure table cell @table.align[alignment] and @table.fill[fill] per column
- Fixed automatic figure @figure.kind[`kind`] detection
- Made alignment of @enum.number-align[enum numbers] configurable, defaulting to `end`
- Figures can now be made breakable with a show-set rule for blocks in figure
- Initial fix for smart quotes in RTL languages

= Export <export>
- Fixed ligatures in PDF export: They are now copyable and searchable
- Exported PDFs now embed ICC profiles for images that have them
- Fixed export of strokes with zero thickness

= Web app <web-app>
- Projects can now contain folders
- Added upload by drag-and-drop into the file panel
- Files from the file panel can now be dragged into the editor to insert them into a Typst file
- You can now copy-paste images and other files from your computer directly into the editor
- Added a button to resend confirmation email
- Added an option to invert preview colors in dark mode
- Added tips to the loading screen and the Help menu. Feel free to propose more!
- Added syntax highlighting for YAML files
- Allowed middle mouse button click on many buttons to navigate into a new tab
- Allowed more project names
- Fixed overridden Vim mode keybindings
- Fixed many bugs regarding file upload and more

= Miscellaneous Improvements <miscellaneous-improvements>
- Improved performance of counters, state, and queries
- Improved incremental parsing for more efficient recompilations
- Added support for `.yaml` extension in addition to `.yml` for bibliographies
- The CLI now emits escape codes only if the output is a TTY
- For users of the `typst` crate: The `Document` is now `Sync` again and the `World` doesn't have to be `'static` anymore
