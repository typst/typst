// Test longdivision.

--- math-longdivision-basic ---
// Test with normal dividend.
$ 32 longdivision(252.8) quad
  5 longdivision(12345) \
  x - 3 longdivision(x^2 + 3x - 5) $

--- math-longdivision-tall ---
// Test with tall dividend.
$ longdivision(X^X^X^X) quad
  longdivision(a/b) $
