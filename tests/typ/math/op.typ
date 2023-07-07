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
#set page(width: auto)
#set text(font: "New Computer Modern")
Discuss $lim_(n->oo) 1/n$ now.
$ lim_(n->infinity) 1/n = 0 $

---
// Test custom operator.
$ op("myop", limits: #false)_(x:=1) x \
  op("myop", limits: #true)_(x:=1) x $

---
// Test styled operator.
$ bold(op("bold", limits: #true))_x y $
