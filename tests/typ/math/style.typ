// Test text styling in math.

---
// Test italic defaults.
$a, A, delta, ϵ, diff, Delta, ϴ$

---
// Test forcing a specific style.
$A, italic(A), upright(A), bold(A), bold(upright(A)), \
 serif(A), sans(A), cal(A), frak(A), mono(A), bb(A), \
 italic(diff), upright(diff), \
 bb("hello") + bold(cal("world")), \
 mono("SQRT")(x) wreath mono(123 + 456)$

---
// Test forcing math size
$a/b, display(a/b), display(a)/display(b), inline(a/b), script(a/b), sscript(a/b) \
 mono(script(a/b)), script(mono(a/b))\
 script(a^b, cramped: #true), script(a^b, cramped: #false)$

---
// Test a few style exceptions.
$h, bb(N), cal(R), Theta, italic(Theta), sans(Theta), sans(italic(Theta))$

---
// Test font fallback.
$ よ and 🏳️‍🌈 $

---
// Test text properties.
$text(#red, "time"^2) + sqrt("place")$

---
// Test different font.
#show math.equation: set text(font: "Fira Math")
$ v := vec(1 + 2, 2 - 4, sqrt(3), arrow(x)) + 1 $

---
// Test using rules for symbols
#show sym.tack: it => $#h(1em) it #h(1em)$
$ a tack b $
