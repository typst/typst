#import "utils.typ": *

= Library <library>

== Text <text>
- Updated New Computer Modern fonts to version 8.1.1, fixing a bug where the regular weight of the math font was still using the old @math.cal[calligraphic letterforms] that were supposed to live in @text.stylistic-set[stylistic set] 6 #pr(8552)

== Math <math>
- Fixed a regression where alignment points did not work correctly when placed within @math.lr[`lr`] / matched delimiters #pr(8566)
- Fixed a regression where @math.op[`op`] elements could be vertically misaligned #pr(8546)

== Layout <layout>
- Fixed a bug where gaps could appear in multi-page lists with @enum.number-align[`number-align`] / @list.marker-align[`marker-align`] set to an alignment with a vertical component #pr(8649)

= Export <export>

== SVG <svg>
- Fixed that inline SVGs resulting from @html.frame elements did not respect pretty printing #pr(8535)

== Bundle <bundle>
- In the watch server, the automatic selection of appropriate `Content-Type` headers based on file extensions was expanded to include more commonly used file types (e.g. `json` files) #pr(8650)
- The error for PNG/SVG documents with multiple pages is now only raised if it persists until the final document iteration #pr(8618)

= Command Line Interface <command-line-interface>
- The `typst eval` subcommand will now exit with code 1 when the provided expression fails to evaluate #pr(8623)
