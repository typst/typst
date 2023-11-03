// Test spacing in math formulas.

---
// Test spacing cases.
$ä, +, c, (, )$ \
$=), (+), {times}$
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
---
// Test ignored vs non-ignored spaces.
$f (x), f(x)$ \
$[a|b], [a | b]$ \
$a"is"b, a "is" b$

---
// Test predefined spacings.
$a thin b, a med b, a thick b, a quad b$ \
$a = thin b$ \
$a - b equiv c quad (mod 2)$

---
// Test spacing for set comprehension.
#set page(width: auto)
$ { x in RR | x "is natural" and x < 10 } $

---
// Test spacing for operators with decorations and modifiers on them
#set page(width: auto)
$a equiv b + c - d => e log 5 op("ln") 6$ \
$a cancel(equiv) b overline(+) c arrow(-) d hat(=>) e cancel(log) 5 dot(op("ln")) 6$ \
$a overbrace(equiv) b underline(+) c grave(-) d underbracket(=>) e circle(log) 5 caron(op("ln")) 6$ \
\
$a attach(equiv, tl: a, tr: b) b attach(limits(+), t: a, b: b) c tilde(-) d breve(=>) e attach(limits(log), t: a, b: b) 5 attach(op("ln"), tr: a, bl: b) 6$
---
// Test weak spacing
$integral f(x) dif x$,
// Not weak
$integral f(x) thin dif x$,
// Both are weak, collide
$integral f(x) #h(0.166em, weak: true)dif x$
