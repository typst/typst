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
$

---
// Test alternating alignment.
#set page(width: 300pt)
$
"abc" & "abc abc abc" & "abc abc" \
"abc abc" & "abc abc" & "abc" \
"abc abc abc" & "abc" & "abc abc abc" \
$
