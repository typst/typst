// Test matrices.

---
// Test semicolon syntax.
#set align(center)
$mat() dot.op
 mat(;) dot.op
 mat(1, 2) dot.op
 mat(1, 2;) \
 mat(1; 2) dot.op
 mat(1, 2; 3, 4) dot.op
 mat(1 + &2, 1/2; &3, 4)$

---
// Test sparse matrix.
$ mat(
  1, 2, ..., 10;
  2, 2, ..., 10;
  dots.v, dots.v, dots.down, dots.v;
  10, 10, ..., 10;
) $

---
// Test baseline alignment.
$ mat(
    a, b^2;
    sum_(x \ y) x, a^(1/2);
    zeta, alpha;
  ) $

---
// Test alternative delimiter with set rule.
#set math.mat(delim: "[")
$ mat(1, 2; 3, 4) $
$ a + mat(delim: #none, 1, 2; 3, 4) + b $

---
// Test alternative math delimiter directly in call.
#set align(center)
#grid(
  columns: 3,
  gutter: 10pt,

  $ mat(1, 2, delim: "[") $,
  $ mat(1, 2; delim: "[") $,
  $ mat(delim: "[", 1, 2) $,

  $ mat(1; 2; delim: "[") $,
  $ mat(1; delim: "[", 2) $,
  $ mat(delim: "[", 1; 2) $,

  $ mat(1, 2; delim: "[", 3, 4) $,
  $ mat(delim: "[", 1, 2; 3, 4) $,
  $ mat(1, 2; 3, 4; delim: "[") $,
)

---
// Error: 13-14 expected array, found content
$ mat(1, 2; 3, 4, delim: "[") $,
