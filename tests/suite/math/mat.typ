// Test matrices.

--- math-mat-semicolon render ---
// Test semicolon syntax.
#set align(center)
$mat() dot
 mat(;) dot
 mat(1, 2) dot
 mat(1, 2;) \
 mat(1; 2) dot
 mat(1, 2; 3, 4) dot
 mat(1 + &2, 1/2; &3, 4)$

--- math-mat-sparse render ---
// Test sparse matrix.
$ mat(
  1, 2, ..., 10;
  2, 2, ..., 10;
  dots.v, dots.v, dots.down, dots.v;
  10, 10, ..., 10;
) $

--- math-mat-baseline render ---
// Test baseline alignment.
$ mat(
  a, b^2;
  sum_(x \ y) x, a^(1/2);
  zeta, alpha;
) $

--- math-mat-delim-set render ---
// Test alternative delimiter with set rule.
#set math.mat(delim: "[")
$ mat(1, 2; 3, 4) $
$ a + mat(delim: #none, 1, 2; 3, 4) + b $

--- math-mat-delim-direct render ---
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

--- math-mat-spread render ---
// Test argument spreading in matrix.
$ mat(..#range(1, 5).chunks(2))
  mat(#(..range(2).map(_ => range(2)))) $

#let nums = ((1,) * 5).intersperse(0).chunks(3)
$ mat(..nums, delim: "[") $

--- math-mat-spread-1d render ---
$ mat(..#range(1, 5) ; 1, ..#range(2, 5))
  mat(..#range(1, 3), ..#range(3, 5) ; ..#range(1, 4), 4) $

--- math-mat-spread-2d render ---
#let nums = range(0, 2).map(i => (i, i+1))
$ mat(..nums, delim: "|",)
  mat(..nums; delim: "|",) $
$ mat(..nums) mat(..nums;) \
  mat(..nums;,) mat(..nums,) $

--- math-mat-spread-expected-array-error render ---
#let nums = range(0, 2).map(i => (i, i+1))
// Error: 15-16 expected array, found content
$ mat(..nums, 0, 1) $

--- math-mat-gap render ---
#set math.mat(gap: 1em)
$ mat(1, 2; 3, 4) $

--- math-mat-gaps render ---
#set math.mat(row-gap: 1em, column-gap: 2em)
$ mat(1, 2; 3, 4) $
$ mat(column-gap: #1em, 1, 2; 3, 4)
  mat(row-gap: #2em, 1, 2; 3, 4) $

--- math-mat-augment render ---
// Test matrix line drawing (augmentation).
#grid(
  columns: 2,
  gutter: 10pt,
  $ mat(10, 2, 3, 4; 5, 6, 7, 8; augment: #3) $,
  $ mat(10, 2, 3, 4; 5, 6, 7, 8; augment: #4) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 0)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 2)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 3)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: 1, vline: 1)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(hline: -1, vline: -1)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(vline: 2, stroke: 1pt + blue)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(vline: 3, stroke: 1pt + blue)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(vline: 3, stroke: 1pt + blue)) $,
  $ mat(100, 2, 3; 4, 5, 6; 7, 8, 9; augment: #(vline: 0, stroke: 1pt + blue)) $,
  $ mat(10, 2, 3, 4; 5, 6, 7, 8; augment: #(vline: -4, stroke: 1pt + blue)) $,
)

--- math-mat-augment-set render ---
// Test using matrix line drawing with a set rule.
#set math.mat(augment: (hline: 2, vline: 1, stroke: 2pt + green))
$ mat(1, 0, 0, 0; 0, 1, 0, 0; 0, 0, 1, 1) $

#set math.mat(augment: 2)
$ mat(1, 0, 0, 0; 0, 1, 0, 0; 0, 0, 1, 1) $

#set math.mat(augment: none)

--- math-mat-augment-line-out-of-bounds render ---
// Error: 3-37 cannot draw a vertical line at offset 4 in a matrix with 3 columns
$ mat(1, 0, 0; 0, 1, 1; augment: #4) $,

--- math-mat-align render ---
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1; align: #left) $
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1; align: #center) $
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1; align: #right) $

--- math-mat-align-explicit-alternating render ---
// Test alternating explicit alignment in a matrix.
$ mat(
  "a" & "a a a" & "a a";
  "a a" & "a a" & "a";
  "a a a" & "a" & "a a a";
) $

--- math-mat-align-implicit render ---
// Test alignment in a matrix.
$ mat(
  "a", "a a a", "a a";
  "a a", "a a", "a";
  "a a a", "a", "a a a";
) $

--- math-mat-align-explicit-left render ---
// Test explicit left alignment in a matrix.
$ mat(
  &"a", &"a a a", &"a a";
  &"a a", &"a a", &"a";
  &"a a a", &"a", &"a a a";
) $

--- math-mat-align-explicit-right render ---
// Test explicit right alignment in a matrix.
$ mat(
  "a"&, "a a a"&, "a a"&;
  "a a"&, "a a"&, "a"&;
  "a a a"&, "a"&, "a a a"&;
) $

--- math-mat-align-explicit-mixed render ---
// Test explicit alignment in some columns with align parameter in a matrix.
#let data = (
  ($&18&&.02$, $1$, $+1$),
  ($-&9&&.3$, $-1$, $-&21$),
  ($&&&.011$, $1$, $&0$)
)
$ #math.mat(align: left, ..data) $
$ #math.mat(align: center, ..data) $
$ #math.mat(align: right, ..data) $

--- math-mat-align-complex render ---
// Test #460 equations.
#let stop = {
  math.class("punctuation",$.$)
}
$ mat(&a+b,c;&d, e) $
$ mat(&a+b&,c;&d&, e) $
$ mat(&&&a+b,c;&&&d, e) $
$ mat(stop &a+b&stop,c;...stop stop&d&...stop stop, e) $

--- math-mat-align-signed-numbers render ---
// Test #454 equations.
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $
$ mat(-1&, 1&, 1&; 1&, -1&, 1&; 1&, 1&, -1&) $
$ mat(-1&, 1&, 1&; 1, -1, 1; 1, 1, -1) $
$ mat(&-1, &1, &1; 1, -1, 1; 1, 1, -1) $

--- math-mat-bad-comma render ---
// This error message is bad.
// Error: 13-14 expected array, found content
$ mat(1, 2; 3, 4, delim: "[") $,

--- issue-852-mat-type render ---
$ mat(B, A B) $
$ mat(B, A B, dots) $
$ mat(B, A B, dots;) $
$ mat(#1, #(foo: "bar")) $

--- issue-2268-mat-augment-color render ---
// The augment line should be of the same color as the text
#set text(
  font: "New Computer Modern",
  lang: "en",
  fill: yellow,
)

$mat(augment: #1, M, v) arrow.r.squiggly mat(augment: #1, R, b)$

--- math-mat-delims render ---
$ mat(delim: #none, 1, 2; 3, 4) $

$ mat(delim: "(", 1, 2; 3, 4) $
$ mat(delim: \(, 1, 2; 3, 4) $
$ mat(delim: paren.l, 1, 2; 3, 4) $

$ mat(delim: "[", 1, 2; 3, 4) $
$ mat(delim: \[, 1, 2; 3, 4) $
$ mat(delim: bracket.l, 1, 2; 3, 4) $

$ mat(delim: "⟦", 1, 2; 3, 4) $
$ mat(delim: bracket.stroked.l, 1, 2; 3, 4) $

$ mat(delim: "{", 1, 2; 3, 4) $
$ mat(delim: \{, 1, 2; 3, 4) $
$ mat(delim: brace.l, 1, 2; 3, 4) $

$ mat(delim: "|", 1, 2; 3, 4) $
$ mat(delim: \|, 1, 2; 3, 4) $
$ mat(delim: bar.v, 1, 2; 3, 4) $

$ mat(delim: "‖", 1, 2; 3, 4) $
$ mat(delim: bar.v.double, 1, 2; 3, 4) $

$ mat(delim: "⟨", 1, 2; 3, 4) $
$ mat(delim: chevron.l, 1, 2; 3, 4) $

--- math-mat-delims-inverted render ---
$ mat(delim: ")", 1, 2; 3, 4) $
$ mat(delim: \), 1, 2; 3, 4) $
$ mat(delim: paren.r, 1, 2; 3, 4) $

$ mat(delim: "]", 1, 2; 3, 4) $
$ mat(delim: \], 1, 2; 3, 4) $
$ mat(delim: bracket.r, 1, 2; 3, 4) $

$ mat(delim: "⟧", 1, 2; 3, 4) $
$ mat(delim: bracket.stroked.r, 1, 2; 3, 4) $

$ mat(delim: "}", 1, 2; 3, 4) $
$ mat(delim: \}, 1, 2; 3, 4) $
$ mat(delim: brace.r, 1, 2; 3, 4) $

$ mat(delim: "⟩", 1, 2; 3, 4) $
$ mat(delim: chevron.r, 1, 2; 3, 4) $

--- math-mat-delims-pair render ---
$ mat(delim: #(none, "["), 1, 2; 3, 4) $
$ mat(delim: #(sym.chevron.r, sym.bracket.stroked.r), 1, 2; 3, 4) $

--- math-mat-linebreaks render ---
// Warning: 20-29 linebreaks are ignored in cells
// Hint: 20-29 use commas instead to separate each line
$ mat(a; b; c) mat(a \ b \ c) $

--- math-mat-vec-cases-unity render ---
// Test that matrices, vectors, and cases are all laid out the same.
$ mat(z_(n_p); a^2)
  vec(z_(n_p), a^2)
  cases(reverse: #true, delim: \(, z_(n_p), a^2)
  cases(delim: \(, z_(n_p), a^2) $

--- issue-1617-mat-align render ---
#set page(width: auto)
$ mat(a, b; c, d) mat(x; y) $

$ x mat(a; c) + y mat(b; d)
  = mat(a x+b y; c x+d y) $

$ mat(
    -d_0, lambda_0, 0, 0, dots;
    mu_1, -d_1, lambda_1, 0, dots;
    0, mu_2, -d_2, lambda_2, dots;
    dots.v, dots.v, dots.v, dots.v, dots.down;
  )
  mat(p_0; p_1; p_2; dots.v) $
