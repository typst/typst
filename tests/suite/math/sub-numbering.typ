// Test sub-numbering of multiline equations.

--- math-sub-numbering-basic paged ---
// Test basic sub-numbering with default settings.
#set math.equation(numbering: "(1)", sub-number: true)

$
  E & = m c^2 \
    & = p c + ... \
    & = sqrt(p^2 c^2 + m^2 c^4)
$

--- math-sub-numbering-disabled paged ---
// Test that sub-numbering can be disabled.
#set math.equation(numbering: "(1)", sub-number: false)

$
  E & = m c^2 \
    & = p c + ... \
    & = ...
$

--- math-sub-numbering-single-line paged ---
// Test that single-line equations are not affected by sub-numbering.
#set math.equation(numbering: "(1)", sub-number: true)

$ F = m a $

--- math-sub-numbering-manual-enable paged ---
// Test manual enabling of sub-numbering for specific lines.
#set math.equation(numbering: "(1)", sub-number: false)

$
  E & = m c^2     & #[#math.line(numbered: true)] \
    & = p c + ... & #[#math.line(numbered: true)] \
    & = ...
$

--- math-sub-numbering-manual-disable paged ---
// Test manual disabling of sub-numbering for specific lines.
#set math.equation(numbering: "(1)", sub-number: true)

$
  E & = m c^2     & #[#math.line(numbered: false)] \
    & = p c + ... & #[#math.line(numbered: false)] \
    & = ...
$

--- math-sub-numbering-alignment paged ---
// Test sub-numbering alignment options.
#set math.equation(numbering: "(1)", sub-number: true, sub-number-align: start)

$
  a & = b \
    & = c
$

#set math.equation(sub-number-align: end)
$
  a & = b \
    & = c
$

--- math-sub-numbering-with-pagebreak paged ---
// Test sub-numbering with page breaking.
#set page(height: 5em)
#set math.equation(numbering: "(1)", sub-number: true)
#show math.equation: set block(breakable: true)

$
  a & + b \
    & + c \
    & + d \
    & = 0
$

--- math-sub-numbering-multiple-equations paged ---
// Test sub-numbering with multiple equations.
#set math.equation(numbering: "(1)", sub-number: true)

$
  a & = b \
    & = c
$

$x + y = z$

$
  p & = q \
    & = r \
    & = s
$

--- math-sub-numbering-empty-lines paged ---
// Test sub-numbering with empty lines in equation.
#set math.equation(numbering: "(1)", sub-number: true)

$
  a & = b \
    & \
    & = c \
$


--- math-sub-numbering-num-dot-num paged ---
// test number sub-numbering (1.1, 1.2)
#set math.equation(numbering: "(1)", sub-number: true, sub-numbering: "(1.1)")

$
  E & = m c^2 \
    & = p c + ... \
    & = ...
$

--- math-sub-numbering-num-dot-alpha paged ---
// Dot separator (1.a, 1.b)
#set math.equation(numbering: "(1)", sub-number: true, sub-numbering: ".a")
$
  E & = m c^2 \
    & = p c + ... \
    & = ...
$

--- math-sub-numbering-num-upper-alpha paged ---
// Uppercase letter (1A, 1B)
#set math.equation(numbering: "(1)", sub-number: true, sub-numbering: "A")
$
  E & = m c^2 \
    & = p c + ... \
    & = ...
$

--- math-sub-numbering-join-number paged ---
// Multiple equations with numbers
#set math.equation(numbering: "(1)", sub-number: true, sub-numbering: "-1")
$
  a & = b \
    & = c
$

$
  x & = y \
    & = z
$

--- math-sub-numbering-reference paged ---
// Test referencing sub-equations with labels.
#set math.equation(numbering: "(1)", sub-number: true)

$
  E & = m c^2     & #[#math.line() <einstein>] \
    & = p c + ... &   #[#math.line() <approx>]
$

See @einstein for the energy-mass relation.
