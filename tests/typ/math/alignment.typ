// Test implicit alignment math.

---
// Test alignment step functions.
$
"a" &= c \
&= c + 1 & "By definition" \
&= d + 100 + 1000 \
&= x && "Even longer" \
$

---
// Test post-fix alignment.
$
& "right" \
"a very long line" \
"left" \
$

---
// Test no alignment.
$
"right" \
"a very long line" \
"left" \
$

---
// Test alternating alignment.
$
"a" & "a a a" & "a a" \
"a a" & "a a" & "a" \
"a a a" & "a" & "a a a" \
$

---
// Test alternating alignment in a vector.
$ vec(
  "a" & "a a a" & "a a",
  "a a" & "a a" & "a",
  "a a a" & "a" & "a a a",
) $

---
// Test alternating explicit alignment in a matrix.
$ mat(
  "a" & "a a a" & "a a";
  "a a" & "a a" & "a";
  "a a a" & "a" & "a a a";
) $

---
// Test alignment in a matrix.
$ mat(
  "a", "a a a", "a a";
  "a a", "a a", "a";
  "a a a", "a", "a a a";
) $

---
// Test explicit left alignment in a matrix.
$ mat(
  &"a", &"a a a", &"a a";
  &"a a", &"a a", &"a";
  &"a a a", &"a", &"a a a";
) $

---
// Test explicit right alignment in a matrix.
$ mat(
  "a"&, "a a a"&, "a a"&;
  "a a"&, "a a"&, "a"&;
  "a a a"&, "a"&, "a a a"&;
) $

---
// Test #460 equations.
$
a &=b & quad c&=d \
e &=f & g&=h
$

$ mat(&a+b,c;&d, e) $
$ mat(&a+b&,c;&d&, e) $
$ mat(&&&a+b,c;&&&d, e) $
$ mat(.&a+b&.,c;.....&d&....., e) $

---
// Test #454 equations.
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $
$ mat(-1&, 1&, 1&; 1&, -1&, 1&; 1&, 1&, -1&) $
$ mat(-1&, 1&, 1&; 1, -1, 1; 1, 1, -1) $
$ mat(&-1, &1, &1; 1, -1, 1; 1, 1, -1) $
