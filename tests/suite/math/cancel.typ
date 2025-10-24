// Tests the cancel() function.

--- math-cancel-inline render ---
// Inline
$a + 5 + cancel(x) + b - cancel(x)$

$c + (a dot.c cancel(b dot.c c))/(cancel(b dot.c c))$

--- math-cancel-display render ---
// Display
#set page(width: auto)
$ a + b + cancel(b + c) - cancel(b) - cancel(c) - 5 + cancel(6) - cancel(6) $
$ e + (a dot.c cancel((b + c + d)))/(cancel(b + c + d)) $

--- math-cancel-inverted render ---
// Inverted
$a + cancel(x, inverted: #true) - cancel(x, inverted: #true) + 10 + cancel(y) - cancel(y)$
$ x + cancel("abcdefg", inverted: #true) $

--- math-cancel-cross render ---
// Cross
$a + cancel(b + c + d, cross: #true, stroke: #red) + e$
$ a + cancel(b + c + d, cross: #true) + e $

--- math-cancel-customized render ---
// Resized and styled
#set page(width: 200pt, height: auto)
$a + cancel(x, length: #200%) - cancel(x, length: #50%, stroke: #(red + 1.1pt))$
$ b + cancel(x, length: #150%) - cancel(a + b + c, length: #50%, stroke: #(blue + 1.2pt)) $

--- math-cancel-angle-absolute render ---
// Specifying cancel line angle with an absolute angle
$cancel(x, angle: #0deg) + cancel(x, angle: #45deg) + cancel(x, angle: #90deg) + cancel(x, angle: #135deg)$

--- math-cancel-angle-func render ---
// Specifying cancel line angle with a function
$x + cancel(y, angle: #{angle => angle + 90deg}) - cancel(z, angle: #(angle => angle + 135deg))$
$ e + cancel((j + e)/(f + e)) - cancel((j + e)/(f + e), angle: #(angle => angle + 30deg)) $
