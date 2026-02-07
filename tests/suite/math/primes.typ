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

--- math-primes-merge-top paged ---
// Test prime attachment merging with the top field.
#set page(width: auto)
$
               attach(a, tr: ', t: b)
  &quad attach(attach(a, tr: '), t: b)
  &quad attach(attach(a, t: b), tr: ')
  &quad attach(attach(attach(a, tl: '), t: b), tr: ')
  &quad attach(attach(attach(a, tr: '), t: b), tr: ')
  \
  // When the base has limits, top prime merging is invariant of t/tr order.
               attach(product, tr: ', t: b)
  &quad attach(attach(product, tr: '), t: b)
  &quad attach(attach(product, t: b), tr: ')
  &quad attach(attach(attach(product, tl: '), t: b), tr: ')
  &quad attach(attach(attach(product, tr: '), t: b), tr: ')
$

--- math-primes-merge-top-nested paged ---
// Test prime-top merging with an additional inner attachment.
// The first row should attach as prime-2, the second row as 2-prime.
$
                     attach(a, b: 1, tr: ', t: 2)
  quad        attach(attach(b, b: 1, tr: '), t: 2)
  quad        attach(attach(c, b: 1), tr: ', t: 2)
  quad attach(attach(attach(d, b: 1), tr: '), t: 2)
  \
              attach(attach(e, b: 1, t: 2), tr: ')
  quad attach(attach(attach(f, b: 1), t: 2), tr: ')
$

--- math-primes-merge-inner-prime paged ---
// Don't join t and tr when there is an outer tr prime.
$
  attach(attach(a, tr: '), t: b, tr: c)
  quad
  attach(attach(a, tr: ', t: b), tr: ')
  quad
  attach(attach(a, tr: c, t: b), tr: ')
$

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
