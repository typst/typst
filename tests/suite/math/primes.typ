--- math-primes paged ---
// Test dedicated syntax for primes
$a'$, $a'''_b$, $'$, $'''''''$

--- math-primes-spaces paged ---
// Test spaces between
$a' ' '$, $' ' '$, $a' '/b$

--- math-primes-complex paged ---
// Test complex prime combinations
$a'_b^c$, $a_b'^c$, $a_b^c'$, $a_b'^c'^d'$

$(a'_b')^(c'_d')$, $a'/b'$, $a_b'/c_d'$

$∫'$, $∑'$, $a'^2^2$, $a'_2_2$

$f_n'^a'$, $f^a'_n'$

$ ∑'_S' $

--- math-primes-attach paged ---
// Test attaching primes only
$a' = a^', a_', a_'''^''^'$

--- math-primes-factorial paged ---
// Test edge cases with factorials and primes
$
  n'!' quad n' !' quad a_n'!'^b \
  n!'! quad n! '! quad a_n!'!^b \
$

--- math-primes-scripts paged ---
// Test primes always attaching as scripts
$ x' $
$ x^' $
$ attach(x, t: ') $
$ <' $
$ attach(<, br: ') $
$ op(<, limits: #true)' $
$ limits(<)' $

--- math-primes-limits paged ---
// Test forcefully attaching primes as limits
$ attach(<, t: ') $
$ <^' $
$ attach(<, b: ') $
$ <_' $

$ limits(x)^' $
$ attach(limits(x), t: ') $

--- math-primes-after-code-expr paged ---
// Test prime symbols after code mode.
#let g = $f$
#let gg = $f$

$
  #(g)' #g' #g ' \
  #g''''''''''''''''' \
  gg'
$

--- math-primes-with-superscript paged ---
// Test prime symbols don't raise the superscript position
$
  sqrt(f)/f
  sqrt(f^2)/f^2
  sqrt(f'^2)/f'^2
  sqrt(f''_n^2)/f''^2_n
$
