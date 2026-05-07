#import "utils.typ": *

= Package Management <package-management>
- Typst now has built-in @reference:scripting:packages[package management]
- You can import #link("https://typst.app/universe")[published] community packages or create and use #link("https://github.com/typst/packages#local-packages")[system-local] ones
- Published packages are also supported in the web app

= Math <math>
- Added support for optical size variants of glyphs in math mode
- Added argument to enable @math.limits[`limits`] conditionally depending on whether the equation is set in @math.display[`display`] or @math.inline[`inline`] style
- Added `gt.eq.slant` and `lt.eq.slant` symbols
- Increased precedence of factorials in math mode (`[$1/n!$]` works correctly now)
- Improved @math.underline[underlines] and @math.overline[overlines] in math mode
- Fixed usage of @math.limits[`limits`] function in show rules
- Fixed bugs with line breaks in equations

= Text and Layout <text-and-layout>
- Added support for alternating page @page.margin[margins] with the `inside` and `outside` keys
- Added support for specifying the page @page.binding[`binding`]
- Added @pagebreak.to[`to`] argument to pagebreak function to skip to the next even or odd page
- Added basic i18n for a few more languages (TR, SQ, TL)
- Fixed bug with missing table row at page break
- Fixed bug with @underline[underlines]
- Fixed bug superfluous table lines
- Fixed smart quotes after line breaks
- Fixed a crash related to text layout

= Command line interface <command-line-interface>
- *Breaking change:* Added requirement for `--root`/`TYPST_ROOT` directory to contain the input file because it designates the _project_ root. Existing setups that use `TYPST_ROOT` to emulate package management should switch to #link("https://github.com/typst/packages#local-packages")[local packages]
- *Breaking change:* Now denies file access outside of the project root
- Added support for local packages and on-demand package download
- Now watches all relevant files, within the root and all packages
- Now displays compilation time

= Miscellaneous Improvements <miscellaneous-improvements>
- Added @outline.entry to customize outline entries with show rules
- Added some hints for error messages
- Added some missing syntaxes for @raw highlighting
- Improved rendering of rotated images in PNG export and web app
- Made @footnote[footnotes] reusable and referenceable
- Fixed bug with citations and bibliographies in @locate
- Fixed inconsistent tense in documentation

= Development <development>
- Added #link("https://github.com/typst/typst/blob/main/CONTRIBUTING.md")[contribution guide]
- Reworked `World` interface to accommodate for package management and make it a bit simpler to implement _(Breaking change for implementors)_
