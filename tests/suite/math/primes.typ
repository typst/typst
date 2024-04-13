--- math-primes ---
// Test dedicated syntax for primes
$a'$, $a'''_b$, $'$, $'''''''$

--- math-primes-spaces ---
// Test spaces between
$a' ' '$, $' ' '$, $a' '/b$

--- math-primes-complex ---
// Test complex prime combinations
$a'_b^c$, $a_b'^c$, $a_b^c'$, $a_b'^c'^d'$

$(a'_b')^(c'_d')$, $a'/b'$, $a_b'/c_d'$

$∫'$, $∑'$, $ ∑'_S' $

--- math-primes-attach ---
// Test attaching primes only
$a' = a^', a_', a_'''^''^'$

--- math-primes-scripts ---
// Test primes always attaching as scripts
$ x' $
$ x^' $
$ attach(x, t: ') $
$ <' $
$ attach(<, br: ') $
$ op(<, limits: #true)' $
$ limits(<)' $

--- math-primes-limits ---
// Test forcefully attaching primes as limits
$ attach(<, t: ') $
$ <^' $
$ attach(<, b: ') $
$ <_' $

$ limits(x)^' $
$ attach(limits(x), t: ') $

--- math-primes-after-code-expr ---
// Test prime symbols after code mode.
#let g = $f$
#let gg = $f$

$
  #(g)' #g' #g ' \
  #g''''''''''''''''' \
  gg'
$
