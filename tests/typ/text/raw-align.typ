// Test the alignment of text inside raw blocks.

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
    align: right,
    block: true,
    lang: "typ",
    "#let f(x) = x\n#align(center, line(length: 1em))"
))
#lorem(20)

---
// 'align: auto' inherits alignment from the context.
#set page(width: 180pt)
#set text(6pt)
#lorem(20)
#align(right, raw(
    align: auto,
    block: true,
    lang: "typ",
    "#let f(x) = x\n#align(center, line(length: 1em))"
))
#lorem(20)