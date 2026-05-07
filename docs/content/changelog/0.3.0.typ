#import "utils.typ": *

= Breaking changes <breaking-changes>
- Renamed a few symbols: What was previous `dot.op` is now just `dot` and the basic dot is `dot.basic`. The same applies to `ast` and `tilde`.
- Renamed `mod` to @calc.rem[`rem`] to more accurately reflect the behavior. It will remain available as `mod` until the next update as a grace period.
- A lone underscore is not a valid identifier anymore, it can now only be used in patterns
- Removed `before` and `after` arguments from @query. This is now handled through flexible @selector[selectors] combinator methods
- Added support for @math.attach[attachments] (sub-, superscripts) that precede the base symbol. The `top` and `bottom` arguments have been renamed to `t` and `b`.

= New features <new-features>
- Added support for more complex @stroke[strokes] (configurable caps, joins, and dash patterns)
- Added @math.cancel[`cancel`] function for equations
- Added support for @reference:scripting:bindings[destructuring] in argument lists and assignments
- Added @image.alt[`alt`] text argument to image function
- Added @toml function for loading data from a TOML file
- Added @array.zip[`zip`], @array.sum[`sum`], and @array.product[`product`] methods for arrays
- Added `fact`, `perm`, `binom`, `gcd`, `lcm`, `atan2`, `quo`, `trunc`, and `fract` @calc[calculation] functions

= Improvements <improvements>
- Text in SVGs now displays properly
- Typst now generates a PDF heading outline
- @ref[References] now provides the referenced element as a field in show rules
- Refined linebreak algorithm for better Chinese justification
- Locations are now a valid kind of selector
- Added a few symbols for algebra
- Added Spanish smart quote support
- Added @selector function to turn a selector-like value into a selector on which combinator methods can be called
- Improved some error messages
- The outline and bibliography headings can now be styled with show-set rules
- Operations on numbers now produce an error instead of overflowing

= Bug fixes <bug-fixes>
- Fixed wrong linebreak before punctuation that follows inline equations, citations, and other elements
- Fixed a bug with @arguments[argument sinks]
- Fixed strokes with thickness zero
- Fixed hiding and show rules in math
- Fixed alignment in matrices
- Fixed some alignment bugs in equations
- Fixed grid cell alignment
- Fixed alignment of list marker and enum markers in presence of global alignment settings
- Fixed path closing
- Fixed compiler crash with figure references
- A single trailing line breaks is now ignored in math, just like in text

= Command line interface <command-line-interface>
- Font path and compilation root can now be set with the environment variables `TYPST_FONT_PATHS` and `TYPST_ROOT`
- The output of `typst fonts` now includes the embedded fonts

= Development <development>
- Added instrumentation for debugging and optimization
- Added `--update` flag and `UPDATE_EXPECT` environment variable to update reference images for tests
- You can now run a specific subtest with `--subtest`
- Tests now run on multiple threads
