#import "utils.typ": *

= PDF export <pdf-export>
- Fixed regression where links to labelled elements would sometimes not work correctly #pr(304, repo: "LaurenzV/krilla")
- Fixed bug where PDF text attributes could be written incorrectly #pr(7478)
- Fixed crash in link handling #pr(7471)
- Fixed crash for zero-sized pages #pr(7454)
- Fixed crash when a table @table.vline[`vline`] or @table.hline[`hline`] has an out-of-bounds index #pr(7448)
- Fixed crash in formatting of font-related PDF export errors #pr(7460)
- Fixed crash when a @footnote or @place element was queried and reinserted into the document #pr(7216)
- Fixed crash for PNGs with invalid metadata #pr(286, repo: "LaurenzV/krilla"), #pr(287, repo: "LaurenzV/krilla")
- Fixed bug where text in SVGs with `fill-and-stroke` paint order could be exported incorrectly #pr(292, repo: "LaurenzV/krilla")
- Fixed bug with layer isolation in SVGs where blending/masking is used #pr(295, repo: "LaurenzV/krilla")
- Fixed that table headers could be tagged incorrectly in some scenarios #pr(289, repo: "LaurenzV/krilla")
- Fixed issues where generated PDFs could differ between 32-bit and 64-bit systems #pr(317, repo: "LaurenzV/krilla"), #pr(316, repo: "LaurenzV/krilla"), #pr(312, repo: "LaurenzV/krilla")
- Upgraded JPEG decoder used during PDF export for improved compatibility, fixing a case where a valid JPEG was rejected #pr(288, repo: "LaurenzV/krilla")
- A PDF document information dictionary that would be empty is now fully omitted instead #pr(280, repo: "LaurenzV/krilla")
- A rare crash in PDF tagging was turned into a compiler error #pr(7450)

= HTML export <html-export>
- Fixed export of table @table.header[headers] and @table.footer[footers] with gutter #pr(7332)
- A @page set rule in HTML export is now a warning instead of a hard error, in line with how unsupported elements are generally treated #pr(7513)

= Math <math>
- Fixed regression where `arrow.l.r` could not be used as an @math.accent[accent] anymore #pr(7481)
- Fixed that single-letter strings did not react to spaces around them like multi-letter strings do #pr(7276)
- Fixed that spacing around @math.mat[`mat`] and @math.vec[`vec`] with a fence delimiter was whitespace-dependent #pr(7309)
- Fixed height calculation for horizontally stretched glyphs #pr(7327)

= Model <model>
- Fixed regression where Typst would error in heading numbering functions that don't handle the counter state `{(0,)}`. This can occur in the first layout iteration. Such errors are usually automatically caught by Typst, which was not the case here. #pr(7459)

= Text <text>
- Fixed regression where Typst would synthesize superscripts for some fonts even when @super.typographic[typographic] glyphs were available #pr(7462)
- Fixed regression where some oblique fonts would be classified as italic #pr(7483)

= Scripting <scripting>
- Fixed crash due to violated invariants in @array.sorted #pr(7520)
- Fixed crashes due to overflow in @calc.rem, @calc.rem-euclid, @calc.div-euclid, @calc.quo, and @calc.gcd #pr(7419)
- Upgraded WebAssembly runtime, fixing a bug that @plugin[plugins] could run into #pr(7438)

= Command Line Interface <command-line-interface>
- Compiling to standard output in combination with `--deps --deps-format=make` (which results in an error) will not produce an empty Make dependency file as a side effect anymore #pr(7246)

= Development <development>
- Increased minimum supported Rust version to 1.89 #pr(7363)
