// Test text operators.

---
// Test predefined.
$ max_(1<=n<=m) n $

---
// With or without parens.
$  &sin x + log_2 x \
 = &sin(x) + log_2(x) $

---
// Test scripts vs limits.
#set text("Latin Modern Roman")
Discuss $lim_(n->infty) 1/n$ now.
$ lim_(n->infty) 1/n = 0 $

---
// Test custom operator.
$ op("myop", limits: #false)_(x:=1) x \
  op("myop", limits: #true)_(x:=1) x $
