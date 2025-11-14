// Test under/over things.

--- math-underover-brace paged ---
// Test braces.
$ x = underbrace(
  1 + 2 + ... + 5,
  underbrace("numbers", x + y)
) $

--- math-underover-line-bracket paged ---
// Test lines and brackets.
$ x = overbracket(
  overline(underline(x + y)),
  1 + 2 + ... + 5,
) $

--- math-underover-brackets paged ---
// Test brackets.
$ underbracket([1, 2/3], "relevant stuff")
          arrow.l.r.double.long
  overbracket([4/5,6], "irrelevant stuff") $

--- math-underover-parens paged ---
// Test parentheses.
$ overparen(
  underparen(x + y, "long comment"),
  1 + 2 + ... + 5  
) $

--- math-underover-shells paged ---
// Test tortoise shell brackets.
$ undershell(
  1 + overshell(2 + ..., x + y),
  "all stuff"
) $

--- math-underover-line-subscript paged ---
// Test effect of lines on subscripts.
$A_2 != overline(A)_2 != underline(A)_2 != underline(overline(A))_2 \
 V_y != overline(V)_y != underline(V)_y != underline(overline(V))_y \
 W_l != overline(W)_l != underline(W)_l != underline(overline(W))_l$

--- math-underover-line-superscript paged ---
// Test effect of lines on superscripts.
$J^b != overline(J)^b != underline(J)^b != underline(overline(J))^b \
 K^3 != overline(K)^3 != underline(K)^3 != underline(overline(K))^3 \
 T^i != overline(T)^i != underline(T)^i != underline(overline(T))^i$

--- math-underover-multiline-annotation paged ---
// Test that multiline annotations do not change the baseline.
$ S = overbrace(beta (alpha) S I, "one line")
    - overbrace(mu (N), "two" \  "line") $
$ S = underbrace(beta (alpha) S I, "one line")
    - underbrace(mu (N), "two" \  "line") $
