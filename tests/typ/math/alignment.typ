// Test implicit alignment math.

---
// Test alignment step functions.
#set page(width: 225pt)
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
// Test #460 equations.
$
a &=b & quad c&=d \
e &=f & g&=h
$
