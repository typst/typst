// Test var (mathematical text)

---
// Test show rule
#set text(fill:red)
#show math.var: set text(fill:blue, size:20pt)
$ a < b "iff" b > a $

---
// Test default italic vs. normal
$var("h") quad var("hello") quad italic(var("world"))$

---
// Test regex
#show "p": "ze"
#show sym.sum: "S"
$var("map") quad sum quad var("∑igma")$#h(1em)∑
