// Tests the cancel() function.

---
// Inline
$a + 5 + cancel(x) + b - cancel(x)$

$c + (a dot.c cancel(b dot.c c))/(cancel(b dot.c c))$

---
// Display
#set page(width: auto)
$ a + b + cancel(b + c) - cancel(b) - cancel(c) - 5 + cancel(6) - cancel(6) $
$ e + (a dot.c cancel((b + c + d)))/(cancel(b + c + d)) $

---
// Inverted
$a + cancel(x, inverted: #true) - cancel(x, inverted: #true) + 10 + cancel(y) - cancel(y)$
$ x + cancel("abcdefg", inverted: #true) $

---
// Cross
$a + cancel(b + c + d, cross: #true, stroke: #red) + e$
$ a + cancel(b + c + d, cross: #true) + e $

---
// Resized and styled
#set page(width: 200pt, height: auto)
$a + cancel(x, length: #200%) - cancel(x, length: #50%, stroke: #{red + 1.1pt})$
$ b + cancel(x, length: #150%) - cancel(a + b + c, length: #50%, stroke: #{blue + 1.2pt}) $

---
// Rotated
$x + cancel(y, rotation: #90deg) - cancel(z, rotation: #135deg)$
$ e + cancel((j + e)/(f + e)) - cancel((j + e)/(f + e), rotation: #30deg) $
