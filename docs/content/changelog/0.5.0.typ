#import "utils.typ": *

= Text and Layout <text-and-layout>
- Added @raw syntax highlighting for many more languages
- Added support for Korean @numbering[numbering]
- Added basic i18n for a few more languages (NL, SV, DA)
- Improved line breaking for East Asian languages
- Expanded functionality of outline @outline.indent[`indent`] property
- Fixed footnotes in columns
- Fixed page breaking bugs with @footnote[footnotes]
- Fixed bug with handling of footnotes in lists, tables, and figures
- Fixed a bug with CJK punctuation adjustment
- Fixed a crash with rounded rectangles
- Fixed alignment of @line elements

= Math <math>
- *Breaking change:* The syntax rules for mathematical @math.attach[attachments] were improved: `[$f^abs(3)$]` now parses as `[$f^(abs(3))$]` instead of `[$(f^abs)(3)$]`. To disambiguate, add a space: `[$f^zeta (3)$]`.
- Added @math:sizes[forced size] commands for math (e.g., @math.display[`display`])
- Added @math.equation.supplement[`supplement`] parameter to @math.equation[`equation`], used by @ref[references]
- New @sym[symbols]: `bullet`, `xor`, `slash.big`, `sigma.alt`, `tack.r.not`, `tack.r.short`, `tack.r.double.not`
- Fixed a bug with symbols in matrices
- Fixed a crash in the @math.attach[`attach`] function

= Scripting <scripting>
- Added new @datetime type and @datetime.today to retrieve the current date
- Added @str.from-unicode and @str.to-unicode functions
- Added @content.fields[`fields`] method on content
- Added `base` parameter to @str function
- Added @calc.exp and @calc.ln
- Improved accuracy of @calc.pow and @calc.log for specific bases
- Fixed @dictionary.remove[removal] order for dictionary
- Fixed `.at(default: ..)` for @str.at[strings] and @content.at[content]
- Fixed field access on styled elements
- Removed deprecated `calc.mod` function

= Command line interface <command-line-interface>
- Added PNG export via `typst compile source.typ output-{n}.png`. The output path must contain `[{n}]` if the document has multiple pages.
- Added `--diagnostic-format=short` for Unix-style short diagnostics
- Doesn't emit color codes anymore if stderr isn't a TTY
- Now sets the correct exit when invoked with a nonexistent file
- Now ignores UTF-8 BOM in Typst files

= Miscellaneous Improvements <miscellaneous-improvements>
- Improved errors for mismatched delimiters
- Improved error message for failed length comparisons
- Fixed a bug with images not showing up in Apple Preview
- Fixed multiple bugs with the PDF outline
- Fixed citations and other searchable elements in @hide
- Fixed bugs with @ref.supplement[reference supplements]
- Fixed Nix flake
