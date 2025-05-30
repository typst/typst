// Test spacing in math formulas.

--- math-spacing-basic ---
// Test spacing cases.
$ä, +, c, (, )$ \
$=), (+), {times}$ \
$⟧<⟦, abs(-), [=$ \
$a=b, a==b$ \
$-a, +a$ \
$a not b$ \
$a+b, a*b$ \
$sum x, sum(x)$ \
$sum product x$ \
$f(x), zeta(x), "frac"(x)$ \
$a+dots.c+b$
$f(x) sin(y)$

--- math-spacing-kept-spaces ---
// Test ignored vs non-ignored spaces.
$f (x), f(x)$ \
$[a|b], [a | b]$ \
$a"is"b, a "is" b$

--- math-spacing-predefined ---
// Test predefined spacings.
$a thin b, a med b, a thick b, a quad b$ \
$a = thin b$ \
$a - b equiv c quad (mod 2)$

--- math-spacing-set-comprehension ---
// Test spacing for set comprehension.
#set page(width: auto)
$ { x in RR | x "is natural" and x < 10 } $

--- math-spacing-decorated ---
// Test spacing for operators with decorations and modifiers on them
#set page(width: auto)
$a equiv b + c - d => e log 5 op("ln") 6$ \
$a cancel(equiv) b overline(+) c arrow(-) d hat(=>) e cancel(log) 5 dot(op("ln")) 6$ \
$a overbrace(equiv) b underline(+) c grave(-) d underbracket(=>) e circle(log) 5 caron(op("ln")) 6$ \
\
$a attach(equiv, tl: a, tr: b) b attach(limits(+), t: a, b: b) c tilde(-) d breve(=>) e attach(limits(log), t: a, b: b) 5 attach(op("ln"), tr: a, bl: b) 6$

--- math-spacing-weak ---
// Test weak spacing
$integral f(x) dif x$,
// Not weak
$integral f(x) thin dif x$,
// Both are weak, collide
$integral f(x) #h(0.166em, weak: true)dif x$

--- math-spacing-script ---
// Test spacing in script size
$x^(a #h(1em) b) + x^x^(a #h(1em) b) + sscript(a #h(1em) b)$

--- math-spacing-ignorant ---
// Test spacing with ignorant elements
$#metadata(none) "text"$ \
$#place(dx: 5em)[Placed] "text"$ \
// Operator spacing
$#counter("test").update(3) + b$ \
$#place(dx: 5em)[a] + b$
// Validate that ignorant elements are layouted
#context test(counter("test").get(), (3,))

--- math-spacing-relative ---
// Test relative spacing.
$ A #h(50%) B \
  A#block(width: 50%);B \
  A #block(width: 50%) B \
  A space #h(50%) space B $

--- math-spacing-relative-inline ---
// Test relative spacing in inline math.
#let mtext = text.with(font: "Libertinus Serif")
Hello#h(40%)world \
Hello#box(width: 40%);world \
Hello$#h(40%)$world \
Hello$#box(width: 40%)$world \
$mtext("Hello") #h(40%) mtext("world")$ \
$mtext("Hello")#box(width: 40%);mtext("world")$

Hello #h(40%) world \
Hello #box(width: 40%) world \
Hello $#h(40%)$ world \
Hello $#box(width: 40%)$ world \
$mtext("Hello") #h(40%) space mtext("world")$ \
$mtext("Hello") #box(width: 40%) mtext("world")$

--- math-spacing-fractional-inline ---
// Test fractional spacing in inline math.
Hello #h(1fr) world \
Hello $#h(1fr)$ world

x #h(1fr) y \
$x #h(1fr) y$

Blah #h(1.5fr) long$#h(0.5fr) x - #h(1fr) y$ line. \
Blah #h(1.5fr) long $#h(0.5fr) x - #h(1fr) y$ line.

--- math-spacing-mixed-inline ---
// Test mixture of different kinds of spacing in inline math.
Some #h(30%) inline $x + #h(5%) y - #h(1fr) sum_(1 #h(1fr) 2) $ spacing #h(2fr) blah.
Long $(a #h(1fr) z) #h(1em, weak: true)$ #h(1%) $#h(0.5fr) sqrt(1 + #h(0.5fr) y)$.

--- issue-1052-math-number-spacing ---
// Test spacing after numbers in math.
$
10degree \
10 degree \
10.1degree \
10.1 degree
$
