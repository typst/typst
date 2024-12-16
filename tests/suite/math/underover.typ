// Test under/over things.

--- math-underover-brace ---
// Test braces.
$ x = underbrace(
  1 + 2 + ... + 5,
  underbrace("numbers", x + y)
) $

--- math-underover-line-bracket ---
// Test lines and brackets.
$ x = overbracket(
  overline(underline(x + y)),
  1 + 2 + ... + 5,
) $

--- math-underover-brackets ---
// Test brackets.
$ underbracket([1, 2/3], "relevant stuff")
          arrow.l.r.double.long
  overbracket([4/5,6], "irrelevant stuff") $

--- math-underover-parens ---
// Test parentheses.
$ overparen(
  underparen(x + y, "long comment"),
  1 + 2 + ... + 5  
) $

--- math-underover-shells ---
// Test tortoise shell brackets.
$ undershell(
  1 + overshell(2 + ..., x + y),
  "all stuff"
) $

--- math-underover-line-subscript ---
// Test effect of lines on subscripts.
$A_2 != overline(A)_2 != underline(A)_2 != underline(overline(A))_2 \
 V_y != overline(V)_y != underline(V)_y != underline(overline(V))_y \
 W_l != overline(W)_l != underline(W)_l != underline(overline(W))_l$

--- math-underover-line-superscript ---
// Test effect of lines on superscripts.
$J^b != overline(J)^b != underline(J)^b != underline(overline(J))^b \
 K^3 != overline(K)^3 != underline(K)^3 != underline(overline(K))^3 \
 T^i != overline(T)^i != underline(T)^i != underline(overline(T))^i$

--- math-underover-multiline-annotation ---
// Test that multiline annotations do not change the baseline.
$ S = overbrace(beta (alpha) S I, "one line")
    - overbrace(mu (N), "two" \  "line") $
$ S = underbrace(beta (alpha) S I, "one line")
    - underbrace(mu (N), "two" \  "line") $
