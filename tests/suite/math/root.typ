// Test roots.

--- math-root-basic ---
// Test root with more than one character.
$A = sqrt(x + y) = c$

--- math-root-radical-attachment ---
// Test root size with radicals containing attachments.
$ sqrt(a) quad
  sqrt(f) quad
  sqrt(q) quad
  sqrt(a^2) \
  sqrt(n_0) quad
  sqrt(b^()) quad
  sqrt(b^2) quad
  sqrt(q_1^2) $

--- math-root-precomposed ---
// Test precomposed vs constructed roots.
// 3 and 4 are precomposed.
$sqrt(x)$
$root(2, x)$
$root(3, x)$
$root(4, x)$
$root(5, x)$

--- math-root-large-body ---
// Test large bodies
$ sqrt([|x|]^2 + [|y|]^2) < [|z|] $
$ v = sqrt((1/2) / (4/5))
   = root(3, (1/2/3) / (4/5/6))
   = root(4, ((1/2) / (3/4)) / ((1/2) / (3/4))) $
$ v = sqrt(a +\ b) $

--- math-root-large-index ---
// Test large index.
$ root(2, x) quad
  root(3/(2/1), x) quad
  root(1/11, x) quad
  root(1/2/3, 1) $

--- math-root-syntax ---
// Test shorthand.
$ √2^3 = sqrt(2^3) $
$ √(x+y) quad ∛x quad ∜x $
$ (√2+3) = (sqrt(2)+3) $

--- math-root-syntax-prec ---
// Precedence of root syntax with other math operators.
$ √a/b ∛a_b ∜f' √n! \
  √a b^c  √a (b)^c  √a(b)^c $

--- math-root-frame-size-index ---
// Test size of final frame when there is an index.
$ a root(, 3)         & a root(., 3) \
  a sqrt(3)           & a root(2, 3) \
  a root(#h(-1em), 3) & a root(123, 3) $
