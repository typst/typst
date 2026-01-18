// Complex wrap-float benchmark: multiple floats
// Expected: < 50% regression vs baseline
#set page(height: 600pt, width: 400pt)
#set par(justify: true)

= Multiple Wrap-Floats

#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 60pt, height: 80pt, fill: aqua.lighten(50%)))

#place(top + left, float: true, wrap: true, dy: 120pt, clearance: 8pt,
  rect(width: 60pt, height: 70pt, fill: teal.lighten(50%)))

#place(top + right, float: true, wrap: true, dy: 240pt, clearance: 8pt,
  rect(width: 60pt, height: 60pt, fill: eastern.lighten(50%)))

This document contains multiple wrap-floats at different positions to measure
the performance impact when the variable-width algorithm handles multiple
exclusion zones.

== Section One

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.

== Section Two

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.

== Section Three

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.

The quick brown fox jumps over the lazy dog. Pack my box with five dozen
liquor jugs. How vexingly quick daft zebras jump. The five boxing wizards
jump quickly. Sphinx of black quartz judge my vow. Two driven jocks help
fax my big quiz. The jay pig fox and zebras quickly moved.
