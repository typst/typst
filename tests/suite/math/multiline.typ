// Test multiline math.

--- math-align-basic paged ---
// Test basic alignment.
$ x &= x + y \
    &= x + 2z \
    &= sum x dot 2z $

--- math-align-wider-first-column paged ---
// Test text before first alignment point.
$ x + 1 &= a^2 + b^2 \
      y &= a + b^2 \
      z &= alpha dot beta $

--- math-align-aligned-in-source paged ---
// Test space between inner alignment points.
$ a + b &= 2 + 3 &= 5 \
      b &= c     &= 3 $

--- math-align-cases paged ---
// Test in case distinction.
$ f := cases(
  1 + 2 &"iff" &x,
  3     &"if"  &y,
) $

--- math-align-lines-mixed paged ---
// Test mixing lines with and some without alignment points.
$ "abc" &= c \
   &= d + 1 \
   = x $

--- math-attach-subscript-multiline paged ---
// Test multiline subscript.
$ sum_(n in NN \ n <= 5) n = (5(5+1))/2 = 15 $

--- math-multiline-no-trailing-linebreak paged ---
// Test no trailing line break.
$
"abc" &= c
$
No trailing line break.

--- math-multiline-trailing-linebreak paged ---
// Test single trailing line break.
$
"abc" &= c \
$
One trailing line break.

--- math-multiline-multiple-trailing-linebreaks paged ---
// Test multiple trailing line breaks.
$
"abc" &= c \ \ \
$
Multiple trailing line breaks.

--- math-linebreaking-after-binop-and-rel paged ---
// Basic breaking after binop, rel
#let hrule(x) = box(line(length: x))
#hrule(45pt)$e^(pi i)+1 = 0$\
#hrule(55pt)$e^(pi i)+1 = 0$\
#hrule(70pt)$e^(pi i)+1 = 0$

--- math-linebreaking-lr paged ---
// LR groups prevent linebreaking.
#let hrule(x) = box(line(length: x))
#hrule(76pt)$a+b$\
#hrule(74pt)$(a+b)$\
#hrule(74pt)$paren.l a+b paren.r$

--- math-linebreaking-multiline paged ---
// Multiline yet inline does not linebreak
#let hrule(x) = box(line(length: x))
#hrule(80pt)$a + b \ c + d$\

--- math-linebreaking-trailing-linebreak paged ---
// A single linebreak at the end still counts as one line.
#let hrule(x) = box(line(length: x))
#hrule(60pt)$e^(pi i)+1 = 0\ $

--- math-linebreaking-in-box paged ---
// Inline, in a box, doesn't linebreak.
#let hrule(x) = box(line(length: x))
#hrule(80pt)#box($a+b$)

--- math-linebreaking-between-consecutive-relations paged ---
// A relation followed by a relation doesn't linebreak
// so essentially `a < = b` can be broken to `a` and `< = b`, `a < =` and `b`
// but never `a <` and `= b` because `< =` are consecutive relation that should
// be grouped together and no break between them.
#let hrule(x) = box(line(length: x))
#hrule(70pt)$a < = b$\
#hrule(78pt)$a < = b$

--- math-linebreaking-after-relation-without-space paged ---
// Line breaks can happen after a relation even if there is no
// explicit space.
#let hrule(x) = box(line(length: x))
#hrule(90pt)$<;$\
#hrule(95pt)$<;$\
// We don't linebreak before a closing paren, but do before an opening paren.
#hrule(90pt)$<($\
#hrule(95pt)$<($
#hrule(90pt)$<)$\
#hrule(95pt)$<)$

--- math-linebreaking-empty paged ---
// Verify empty rows are handled ok.
$ $\
Nothing: $ $, just empty.

--- math-pagebreaking paged ---
// Test breaking of equations at page boundaries.
#set page(height: 5em)
#show math.equation: set block(breakable: true)

$ a &+ b + & c \
  a &+ b   &   && + d \
  a &+ b + & c && + d \
    &      & c && + d \
    &= 0 $

--- math-pagebreaking-numbered paged ---
// Test breaking of equations with numbering.
#set page(height: 5em)
#set math.equation(numbering: "1")
#show math.equation: set block(breakable: true)

$ a &+ b + & c \
  a &+ b   &   && + d \
  a &+ b + & c && + d \
    &      & c && + d \
    &= 0 $

--- math-pagebreaking-single-line paged ---
// Test breaking of single line equations.
#set page(height: 4em)
#show math.equation: set block(breakable: true)

Shouldn't overflow:
$ a + b $

--- math-pagebreaking-single-line-numbered paged ---
// Test breaking of single line equations with numbering.
#set page(height: 4em)
#show math.equation: set block(breakable: true)
#set math.equation(numbering: "(1)")

Shouldn't overflow:
$ a + b $

--- issue-5113-pagebreaking-empty paged ---
// Test empty breakable equations.
#show math.equation: set block(breakable: true)
#math.equation(block: true, [])

--- issue-1948-math-text-break paged ---
// Test text with linebreaks in math.
$ x := "a\nb\nc\nd\ne" $

--- issue-4829-math-pagebreaking-wrong-number paged ---
// Test numbering of empty regions of broken equations.
#set page(height: 5em)
#set math.equation(numbering: "1")
#show math.equation: set block(breakable: true)

#rect(height: 1.5em)

$ a + b \
  a + b $
