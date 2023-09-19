// Test matrices.

---
// Test semicolon syntax.
#set align(center)
$mat() dot
 mat(;) dot
 mat(1, 2) dot
 mat(1, 2;) \
 mat(1; 2) dot
 mat(1, 2; 3, 4) dot
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

---
$ mat(B, A B) $
$ mat(B, A B, dots) $
$ mat(B, A B, dots;) $
$ mat(#1, #(foo: "bar")) $

---

// Test matrix line drawing (augmentation).
#grid(
  columns: 2,
  gutter: 10pt,

  $ mat(10, 2, 3, 4; 5, 6, 7, 8; augment: #3) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 2)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 1, vline: 1)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(vline: 2, stroke: 1pt + blue)) $,
)

---

// Test using matrix line drawing with a set rule.
#set math.mat(augment: (hline: 2, vline: 1, stroke: 2pt + green))
$ mat(1, 0, 0, 0; 0, 1, 0, 0; 0, 0, 1, 1) $

#set math.mat(augment: 2)
$ mat(1, 0, 0, 0; 0, 1, 0, 0; 0, 0, 1, 1) $

#set math.mat(augment: none)

---
// Error: 3-37 cannot draw a vertical line after column 3 of a matrix with 3 columns
$ mat(1, 0, 0; 0, 1, 1; augment: #3) $,
