// Long paragraph wrap-float benchmark
// Expected: < 100% regression vs baseline (worst case for K-P algorithm)
#set page(height: 600pt, width: 400pt)
#set par(justify: true)

= Long Paragraph with Wrap-Float

#place(top + right, float: true, wrap: true, clearance: 8pt,
  rect(width: 40pt, height: 100pt, fill: aqua.lighten(50%)))

This is a long text wrapping around a float. The quick brown fox jumps over
the lazy dog. Pack my box with five dozen jugs. How quick daft zebras jump.
The five boxing wizards jump quickly. Sphinx of black quartz judge my vow.
Two driven jocks help fax my big quiz. The jay pig fox zebras move quickly.
The quick brown fox jumps over the lazy dog. Pack my box with five dozen
jugs. How quick daft zebras jump. The five boxing wizards jump quickly.
Sphinx of black quartz judge my vow. Two driven jocks help fax my big quiz.
The jay pig fox zebras move quickly. The quick brown fox jumps over the lazy
dog. Pack my box with five dozen jugs. How quick daft zebras jump. The five
boxing wizards jump quickly. Sphinx of black quartz judge my vow. Two driven
jocks help fax my big quiz. The jay pig fox zebras move quickly. The quick
brown fox jumps over the lazy dog. Pack my box with five dozen jugs. How
quick daft zebras jump. The five boxing wizards jump quickly.
