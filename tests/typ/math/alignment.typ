// Test implicit alignment math.

---
// Test alignment step functions.
#set page(width: 300pt)
$
"abc" &= c \
&= c + 1 & "By definition" \
&= d + 100 + 1000 \
&= x && "Even longer" \
$

---
// Test post-fix alignment.
#set page(width: 300pt)
$
& "right" \
"a very long line" \
"left" \
$

---
// Test no alignment.
#set page(width: 300pt)
$
"right" \
"a very long line" \
"left" \
$

---
// Test alternating alignment.
#set page(width: 300pt)
$
"abc" & "abc abc abc" & "abc abc" \
"abc abc" & "abc abc" & "abc" \
"abc abc abc" & "abc" & "abc abc abc" \
$

---
// Test alternating alignment in a vector.
#set page(width: 300pt)
$
vec(
"abc" & "abc abc abc" & "abc abc",
"abc abc" & "abc abc" & "abc",
"abc abc abc" & "abc" & "abc abc abc",
)
$

---
// Test alternating explicit alignment in a matrix.
#set page(width: 300pt)
$
mat(
"abc" & "abc abc abc" & "abc abc";
"abc abc" & "abc abc" & "abc";
"abc abc abc" & "abc" & "abc abc abc";
)
$

---
// Test alignment in a matrix.
#set page(width: 300pt)
$
mat(
"abc", "abc abc abc", "abc abc";
"abc abc", "abc abc", "abc";
"abc abc abc", "abc", "abc abc abc";
)
$

---
// Test explicit left alignment in a matrix.
#set page(width: 300pt)
$
mat(
&"abc", &"abc abc abc", &"abc abc";
&"abc abc", &"abc abc", &"abc";
&"abc abc abc", &"abc", &"abc abc abc";
)
$

---
// Test explicit right alignment in a matrix.
#set page(width: 300pt)
$
mat(
"abc"&, "abc abc abc"&, "abc abc"&;
"abc abc"&, "abc abc"&, "abc"&;
"abc abc abc"&, "abc"&, "abc abc abc"&;
)
$

---
// Test #460 equations.
$
a &=b & quad c&=d \
e &=f & g&=h
$

$
mat(&a+b,c;&d, e)
$

$
mat(&a+b&,c;&d&, e)
$

$
mat(&&&a+b,c;&&&d, e)
$

$
mat(.&a+b&.,c;.....&d&....., e)
$

---
// Test #454 equations.
$ mat(-1, 1, 1; 1, -1, 1; 1, 1, -1) $

$ mat(-1&, 1&, 1&; 1&, -1&, 1&; 1&, 1&, -1&) $

$ mat(-1&, 1&, 1&; 1, -1, 1; 1, 1, -1) $

$ mat(&-1, &1, &1; 1, -1, 1; 1, 1, -1) $
