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
$sum prod x$ \
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
