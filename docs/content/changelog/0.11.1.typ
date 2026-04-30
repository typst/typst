#import "utils.typ": *

= Security <security>
- Fixed a vulnerability where image files at known paths could be embedded into the PDF even if they were outside of the project directory

= Bibliography <bibliography>
- Fixed et-al handling in subsequent citations
- Fixed suppression of title for citations and bibliography references with no author
- Fixed handling of initials in citation styles without a delimiter
- Fixed bug with citations in footnotes

= Text and Layout <text-and-layout>
- Fixed interaction of @par.first-line-indent[`first-line-indent`] and @outline
- Fixed compression of CJK punctuation marks at line start and end
- Fixed handling of @rect[rectangles] with negative dimensions
- Fixed layout of `path` in explicitly sized container
- Fixed broken @raw text in right-to-left paragraphs
- Fixed tab rendering in `raw` text with language `typ` or `typc`
- Fixed highlighting of multi-line `raw` text enclosed by single backticks
- Fixed indentation of overflowing lines in `raw` blocks
- Fixed extra space when `raw` text ends with a backtick

= Math <math>
- Fixed broken @math.equation[equations] in right-to-left paragraphs
- Fixed missing @math.bb[blackboard bold] letters
- Fixed error on empty arguments in 2D math argument list
- Fixed stretching via @math.mid[`mid`] for various characters
- Fixed that alignment points in equations were affected by `{set align(..)}`

= Export <export>
- Fixed @smartquote[smart quotes] in PDF outline
- Fixed @tiling[patterns] with spacing in PDF
- Fixed wrong PDF page labels when @page.numbering[page numbering] was disabled after being previously enabled

= Scripting <scripting>
- Fixed overflow for large numbers in external data files (by converting to floats instead)
- Fixed @str.trim[`{str.trim(regex, at: end)}`] when the whole string is matched

= Miscellaneous <miscellaneous>
- Fixed deformed strokes for specific shapes and thicknesses
- Fixed newline handling in code mode: There can now be comments within chained method calls and between an `if` branch and the `else` keyword
- Fixed inefficiency with incremental reparsing
- Fixed autocompletions for relative file imports
- Fixed crash in autocompletion handler
- Fixed a bug where the path and entrypoint printed by `typst init` were not properly escaped
- Fixed various documentation errors
