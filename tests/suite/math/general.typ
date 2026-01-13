// Test general expressions in multiple fonts.

--- math-general-lorenz paged math-fonts ---
// The Lorenz equations.
#set page(width: auto)
$
  dot(x) & = sigma (y - x) \
  dot(y) & = rho x - y - x z \
  dot(z) & = -beta z + x y
$

--- math-general-rogers-ramanujan paged math-fonts ---
// A Rogers-Ramanujan identity.
#set page(width: auto)
$
  1 + q^2 / ((1 - q)) + q^6 / ((1 - q) (1 - q^2)) + dots.c = product_(j = 0)^oo 1 / ((1 - q^(5j + 2)) (1 - q^(5j + 3))), wide "for" quad abs(q) < 1.
$

--- math-general-quadratic paged math-fonts ---
// The solutions to a quadratic equation.
#set page(width: auto)
When $a != 0$, there are two solutions to $a x^2 + b x + c = 0$ and they are
$
  x = (-b plus.minus sqrt(b^2 - 4 a c)) / (2a).
$

--- math-general-cauchy-schwarz paged math-fonts ---
// The Cauchy-Schwarz inequality.
#set page(width: 85mm)
#let inequality = $(sum_(k = 1)^n a_k b_k)^2 <= (sum_(k = 1)^n a_k^2) (sum_(k = 1)^n b_k^2)$

The same inequality holds inline $inequality$ or displayed
$ inequality. $

--- math-general-binomial paged math-fonts ---
// The probability of getting k heads when flipping n coins.
#set page(width: auto)
$
  P(E) = binom(n, k) p^k (1 - p)^(n - k)
$

--- math-general-cross-product paged math-fonts ---
// A cross product formula.
#set page(width: auto)
#let vb(x) = $upright(bold(#x))$
#let pdv(x, y) = $(partial #x) / (partial #y)$
$
  vb(V)_1 times vb(V)_2 = mat(delim: \|, vb(i), vb(j), vb(k); pdv(X, u), pdv(Y, u), 0; pdv(X, v), pdv(Y, v), 0)
$

--- math-general-ramanujan paged math-fonts ---
// An identity of Ramanujan.
#set page(width: auto)
$
  1 / ((sqrt(phi.alt sqrt(5)) - phi.alt) e^(2/5 pi)) = 1 + e^(-2pi) / (1 + e^(-4pi) / (1 + e^(-6pi) / (1 + e^(-8pi) / (1 + dots.c))))
$

--- math-general-maxwell paged math-fonts ---
// Maxwell's equations.
#set page(width: auto)
#let va(x) = $arrow(upright(bold(#x)))$
#let pdv(x, y) = $(partial #x) / (partial #y)$
$
  gradient times va(B) - 1/c pdv(va(E), t) & = (4pi) / c va(j) \
                        gradient dot va(E) & = 4 pi rho \
  gradient times va(E) + 1/c pdv(va(B), t) & = va(0) \
                        gradient dot va(B) & = 0
$

--- math-general-in-line paged math-fonts ---
// In-line mathematics.
#set page(width: 80mm)
Finally, the ability to mix math and text in a paragraph is also important.
This expression $cal(P)'_l approx sqrt(3x - 1) + (1 + x)^2$ is an inline equation.
Equations can be used this way as well, without unduly disturbing the spacing between lines.
