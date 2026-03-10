// Test implicit alignment math.

--- math-align-weird paged ---
// Test alignment step functions.
#set page(width: 225pt)
$
a &= c \
  &= c + 1 & "By definition" \
  &= d + 100 + 1000 \
  &= x && "Even longer" \
$

--- math-align-post-fix paged ---
// Test post-fix alignment.
$
& "right" \
"a very long line" \
"left" \
$

--- math-align-implicit paged ---
// Test no alignment.
$
"right" \
"a very long line" \
"left" \
$

--- math-align-toggle paged ---
// Test #460 equations.
$
a &=b & quad c&=d \
e &=f & g&=h
$

--- issue-3973-math-equation-align paged ---
// In this bug, the alignment set with "show math.equation: set align(...)"
// overrides the left-right alternating behavior of alignment points.
#let equations = [
$ a + b &= c \
      e &= f + g + h $
$         a &= b + c \
  e + f + g &= h $
]
#equations

#show math.equation: set align(start)
#equations

#show math.equation: set align(end)
#equations
