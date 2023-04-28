// Test spacing in math formulas.

---
// Test spacing cases.
$ä, +, c, (, )$ \
$=), (+), {times}$
$⟧<⟦, |-|, [=$ \
$a=b, a==b$ \
$-a, +a$ \
$a not b$ \
$a+b, a*b$ \
$sum x, sum(x)$ \
$sum product x$ \
$f(x), zeta(x), "frac"(x)$

---
// Test ignored vs non-ignored spaces.
$f (x), f(x)$ \
$[a|b], [a | b]$ \
$a"is"b, a "is" b$

---
// Test predefined spacings.
$a thin b, a med b, a thick b, a quad b$ \
$a = thin b$ \
$a - b ident c quad (mod 2)$

---
// Test spacing for set comprehension.
#set page(width: auto)
$ { x in RR | x "is natural" and x < 10 } $

---
// Test spacing for operators with decorations and modifiers on them
#set page(width: auto)
$a ident b + c - d => e log 5 op("ln") 6$ \
$a cancel(ident) b overline(+) c arrow(-) d hat(=>) e cancel(log) 5 dot(op("ln")) 6$ \
$a overbrace(ident) b underline(+) c grave(-) d underbracket(=>) e circle(log) 5 caron(op("ln")) 6$ \
\
$a attach(ident, tl: a, tr: b) b attach(limits(+), t: a, b: b) c tilde(-) d breve(=>) e attach(limits(log), t: a, b: b) 5 attach(op("ln"), tr: a, bl: b) 6$
