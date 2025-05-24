// Test text styling in math.

--- math-style-italic-default ---
// Test italic defaults for all `default-style`s.
$a, A, delta, ϵ, partial, Delta, ϴ$

#set math.equation(default-style: "iso")
$a, A, delta, ϵ, partial, Delta, ϴ$

#set math.equation(default-style: "french")
$a, A, delta, ϵ, partial, Delta, ϴ$

#set math.equation(default-style: "upright")
$a, A, delta, ϵ, partial, Delta, ϴ$

--- math-style ---
// Test forcing a specific style.
$A, italic(A), upright(A), bold(A), bold(upright(A)), \
 serif(A), sans(A), cal(A), frak(A), mono(A), bb(A), \
 italic(partial), upright(partial), \
 bb("hello") + bold(cal("world")), \
 mono("SQRT")(x) wreath mono(123 + 456)$

--- math-style-dotless ---
// Test styling dotless i and j.
$ dotless.i dotless.j,
  upright(dotless.i) upright(dotless.j),
  sans(dotless.i) sans(dotless.j),
  bold(dotless.i) bold(dotless.j),
  bb(dotless.i) bb(dotless.j),
  cal(dotless.i) cal(dotless.j),
  frak(dotless.i) frak(dotless.j),
  mono(dotless.i) mono(dotless.j), 
  bold(frak(dotless.i)) upright(sans(dotless.j)),
  italic(bb(dotless.i)) frak(sans(dotless.j)) $

--- math-style-exceptions ---
// Test a few style exceptions.
$h, bb(N), cal(R), Theta, italic(Theta), sans(Theta), sans(italic(Theta)) \
 bb(d), bb(italic(d)), italic(bb(d)), bb(e), bb(italic(e)), italic(bb(e)) \
 bb(i), bb(italic(i)), italic(bb(i)), bb(j), bb(italic(j)), italic(bb(j)) \
 bb(D), bb(italic(D)), italic(bb(D))$

--- math-style-greek-exceptions ---
// Test a few greek exceptions.
$bb(Gamma) , bb(gamma), bb(Pi), bb(pi), bb(sum)$

--- math-style-hebrew-exceptions ---
// Test hebrew exceptions.
$aleph, beth, gimel, daleth$

--- issue-3650-italic-equation ---
_abc $sin(x) "abc"$_ \
$italic(sin(x) "abc" #box[abc])$ \
*abc $sin(x) "abc"$* \
$bold(sin(x) "abc" #box[abc])$ \
