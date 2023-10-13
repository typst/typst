// Test the alignment of text inside of raw blocks.

---
// Text inside raw block should be unaffected by outer alignment by default.
#set align(center)
#set page(width: 180pt)
#set text(6pt)

#lorem(20)

```py
def something(x):
  return x

a = 342395823859823958329
b = 324923
```

#lorem(20)

---
// Text inside raw block should follow the specified alignment.
#set page(width: 180pt)
#set text(6pt)

#lorem(20)
#align(center, raw(
  lang: "typ",
  block: true,
  align: right,
  "#let f(x) = x\n#align(center, line(length: 1em))",
))
#lorem(20)

---
// Error: 17-20 expected `start`, `left`, `center`, `right`, or `end`, found top
#set raw(align: top)
