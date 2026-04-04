// Mozilla MathML Test formulas from https://mathfonts.github.io/mozilla_mathml_test.

--- mozilla-mathml-test-1 paged html ---
$ x^2 y^2 $

--- mozilla-mathml-test-2 paged html ---
$ attach(F, bl: 2, br: 3) $

--- mozilla-mathml-test-3 paged html ---
$ (x + y^2) / (k + 1) $

--- mozilla-mathml-test-4 paged html ---
$ x + y^(2 / (k + 1)) $

--- mozilla-mathml-test-5 paged html ---
$ a / (b \/ 2) $

--- mozilla-mathml-test-6 paged html ---
$
  a_0 + display(1 / (a_1 + display(1 / (a_2 + display(1 / (a_3 + display(1 / a_4)))))))
$

--- mozilla-mathml-test-7 paged html ---
$ a_0 + 1 / (a_1 + 1 / (a_2 + 1 / (a_3 + 1 / a_4))) $

--- mozilla-mathml-test-8 paged html ---
$ binom(n, k \/ 2) $

--- mozilla-mathml-test-9 paged html ---
$ binom(p, 2) x^2 y^(p - 2) - 1 / (1 - x) 1 / (1 - x^2) $

--- mozilla-mathml-test-10 paged html ---
$ sum_(0 <= i <= m \ 0 < j < n) P(i, j) $

--- mozilla-mathml-test-11 paged html ---
$ x^(2 y) $

--- mozilla-mathml-test-12 paged html ---
$ sum_(i = 1)^p sum_(j = 1)^q sum_(k = 1)^r a_(i j) b_(j k) c_(k i) $

--- mozilla-mathml-test-13 paged html ---
#show: it => context {
  set page(width: auto) if target() == "paged"
  it
}
$ sqrt(1 + sqrt(1 + sqrt(1 + sqrt(1 + sqrt(1 + sqrt(1 + sqrt(1 + x))))))) $

--- mozilla-mathml-test-14 paged html ---
#show: it => context {
  set page(width: auto) if target() == "paged"
  it
}
$
  (partial^2 / (partial x^2) + partial^2 / (partial y^2)) abs(phi(x + i y))^2 = 0
$

--- mozilla-mathml-test-15 paged html ---
$ 2^2^2^x $

--- mozilla-mathml-test-16 paged html ---
$ integral_1^x (dif t) / t $

--- mozilla-mathml-test-17 paged html ---
$ integral.double_D dif x dif y $

--- mozilla-mathml-test-18 paged html ---
$
  f(x) = cases(
    1\/3 & "if" 0 <= x <= 1\;,
    2\/3 & "if" 3 <= x <= 4\;,
    0 & "elsewhere".
  )
$

--- mozilla-mathml-test-19 paged html ---
$ overbrace(x + dots.c + x, k "times") $

--- mozilla-mathml-test-20 paged html ---
$ y_(x^2) $

--- mozilla-mathml-test-21 paged html ---
#show: it => context {
  set page(width: auto) if target() == "paged"
  it
}
$ sum_(p "prime") f(p) = integral_(t > 1) f(t) dif pi(t) $

--- mozilla-mathml-test-22 paged html ---
$
  { underbrace(
      overbrace(a\, ...\, a, k a's)\, overbrace(b\, ...\, b, ell b's),
      k + ell "elements"
    ) }
$

--- mozilla-mathml-test-23 paged html ---
$
  mat(
    mat(a, b; c, d), mat(e, f; g, h);
    0, mat(i, j; k, l);
  )
$

--- mozilla-mathml-test-24 paged html ---
#show: it => context {
  set page(width: auto) if target() == "paged"
  it
}
$
  det mat(
    delim: \|,
    c_0, c_1, c_2, dots.c, c_n;
    c_1, c_2, c_3, dots.c, c_(n + 1);
    c_2, c_3, c_4, dots.c, c_(n + 2);
    dots.v, dots.v, dots.v, , dots.v;
    c_n, c_(n + 1), c_(n + 2), dots.c, c_(2 n);
  ) > 0
$

--- mozilla-mathml-test-25 paged html ---
$ y_x_2 $

--- mozilla-mathml-test-26 paged html ---
$ x_92^31415 + pi $

--- mozilla-mathml-test-27 paged html ---
$ x^(z^d_c)_(y_b^a) $

--- mozilla-mathml-test-28 paged html ---
$ y'''_3 $

--- mozilla-mathml-test-29 paged html ---
$ lim_(n -> +oo) sqrt(2 pi n) / n! (n / e)^n = 1 $

--- mozilla-mathml-test-30 paged html ---
$
  det(A) = sum_(sigma in S_n) epsilon.alt(sigma) product_(i = 1)^n a_(i, sigma_i)
$
