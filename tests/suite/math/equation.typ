// Test alignment of block equations.
// Test show rules on equations.

--- math-equation-numbering ---
#set page(width: 150pt)
#set math.equation(numbering: "(I)")

We define $x$ in preparation of @fib:
$ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>

With @ratio, we get
$ F_n = round(1 / sqrt(5) phi.alt^n) $ <fib>

--- math-equation-font ---
// Test different font.
#show math.equation: set text(font: "Fira Math")
$ v := vec(1 + 2, 2 - 4, sqrt(3), arrow(x)) + 1 $

--- math-equation-show-rule ---
This is small: $sum_(i=0)^n$

#show math.equation: math.display
This is big: $sum_(i=0)^n$

--- math-equation-align-unnumbered ---
// Test unnumbered
#let eq(alignment) = {
  show math.equation: set align(alignment)
  $ a + b = c $
}

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

--- math-equation-align-numbered ---
// Test numbered
#let eq(alignment) = {
  show math.equation: set align(alignment)
  $ a + b = c $
}

#set math.equation(numbering: "(1)")

#eq(center)
#eq(left)
#eq(right)

#set text(dir: rtl)
#eq(start)
#eq(end)

--- math-equation-number-align ---
#set math.equation(numbering: "(1)")

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

--- math-equation-number-align-start ---
#set math.equation(numbering: "(1)", number-align: start)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

--- math-equation-number-align-end ---
#set math.equation(numbering: "(1)", number-align: end)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

--- math-equation-number-align-left ---
#set math.equation(numbering: "(1)", number-align: left)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

--- math-equation-number-align-right ---
#set math.equation(numbering: "(1)", number-align: right)

$ a + b = c $

#show math.equation: set align(center)
$ a + b = c $
#show math.equation: set align(left)
$ a + b = c $
#show math.equation: set align(right)
$ a + b = c $

#set text(dir: rtl)
#show math.equation: set align(start)
$ a + b = c $
#show math.equation: set align(end)
$ a + b = c $

--- math-equation-number-align-center ---
// Error: 52-58 expected `start`, `left`, `right`, or `end`, found center
#set math.equation(numbering: "(1)", number-align: center)

--- math-equation-number-align-center-bottom ---
// Error: 52-67 expected `start`, `left`, `right`, or `end`, found center
#set math.equation(numbering: "(1)", number-align: center + bottom)

--- math-equation-number-align-monoline ---
#set math.equation(numbering: "(1)")
$ p = sum_k k ln a $

#set math.equation(numbering: "(1)", number-align: top)
$ p = sum_k k ln a $

#set math.equation(numbering: "(1)", number-align: bottom)
$ p = sum_k k ln a $

--- math-equation-number-align-multiline ---
#set math.equation(numbering: "(1)")

$ p &= ln a b \
    &= ln a + ln b $

--- math-equation-number-align-multiline-top-start ---
#set math.equation(numbering: "(1)", number-align: top+start)

$ p &= ln a b \
    &= ln a + ln b $
$ q &= sum_k k ln a \
    &= sum_k ln A $

--- math-equation-number-align-multiline-bottom ---
#show math.equation: set align(left)
#set math.equation(numbering: "(1)", number-align: bottom)

$ p &= ln a b \
    &= ln a + ln b $
$ q &= sum_k ln A \
    &= sum_k k ln a $

--- math-equation-number-align-multiline-expand ---
// Tests that if the numbering's layout box vertically exceeds the box of
// the equation frame's boundary, the latter's frame is resized correctly
// to encompass the numbering. #box() below delineates the resized frame.
//
// A row with "-" only has a height that's smaller than the height of the
// numbering's layout box. Note we use pattern "1" here, not "(1)", since
// the parenthesis exceeds the numbering's layout box, due to the default
// settings of top-edge and bottom-edge of the TextElem that laid it out.
#let equations = [
  #box($ - - - $, fill: silver)
  #box(
  $ - - - \
    a = b $,
  fill: silver)
  #box(
  $ a = b \
    - - - $,
  fill: silver)
]

#set math.equation(numbering: "1", number-align: top)
#equations

#set math.equation(numbering: "1", number-align: horizon)
#equations

#set math.equation(numbering: "1", number-align: bottom)
#equations

--- math-equation-number-align-multiline-no-expand ---
// Tests that if the numbering's layout box doesn't vertically exceed the
// box of the equation frame's boundary, the latter's frame size remains.
// So, in the grid below, frames in each row should have the same height.
#set math.equation(numbering: "1")
#grid(
  columns: 4 * (1fr,),
  column-gutter: 3 * (2pt,),
  row-gutter: 2pt,
  align: horizon,
  [
    #set math.equation(number-align: horizon)
    #box($ - - \ a \ sum $, fill: silver)
  ],
  [
    #set math.equation(number-align: bottom)
    #box($ - - \ a \ sum $, fill: silver)
  ],
  [
    #set math.equation(number-align: horizon)
    #box($ sum \ a \ - - $, fill: silver)
  ],
  [
    #set math.equation(number-align: top)
    #box($ sum \ a \ - - $, fill: silver)
  ],

  [
    #set math.equation(number-align: horizon)
    #box($ - - $, fill: silver)
  ],
  [
    #set math.equation(number-align: top)
    #box($ - - $, fill: silver)
  ],
  [
    #set math.equation(number-align: bottom)
    #box($ - - $, fill: silver)
  ],
)

--- math-equation-number-empty ---
// Test numbering on empty equations.
#math.equation(numbering: "1", block: true, [])

--- math-equation-tag-affects-row-height ---
// Tags should not affect the row height of equations.
#box($ - - $, fill: silver)
#box($ #metadata(none) - - $, fill: silver) \
#box($ a \ - - $, fill: silver)
#box($ a \ #metadata(none) - - $, fill: silver)
#box($ - - \ a $, fill: silver)
#box($ #metadata(none) - - \ a $, fill: silver)

--- issue-4187-alignment-point-affects-row-height ---
// In this bug, a row of "-" only should have a very small height; but
// after adding an alignment point "&", the row gains a larger height.
// We need to test alignment point "&" does not affect a row's height.
#box($ - - $, fill: silver)
#box($ - - $, fill: silver) \
#box($ a \ - - $, fill: silver)
#box($ &- - \ &a $, fill: silver)
#box($ &a \ &- - $, fill: silver)

--- issue-numbering-hint ---
// In this bug, the hint and error messages for an equation
// being reference mentioned that it was a "heading" and was
// lacking the proper path.
#set page(height: 70pt)

$
    Delta = b^2 - 4 a c
$ <quadratic>

// Error: 14-24 cannot reference equation without numbering
// Hint: 14-24 you can enable equation numbering with `#set math.equation(numbering: "1.")`
Looks at the @quadratic formula.

--- issue-3696-equation-rtl ---
#set page(width: 150pt)
#set text(lang: "he")
תהא סדרה $a_n$: $[a_n: 1, 1/2, 1/3, dots]$

--- issue-6170-equation-stroke ---
// In this bug stroke settings did not apply to math content.
// We expect all of these to have a green stroke.
#set text(stroke: green + 0.5pt)

A $B^2$ $ grave(C)' $
