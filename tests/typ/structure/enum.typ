// Test enums.

---
. Embrace
. Extend
. Extinguish

---
1. First.
   2. Second.

1. Back to first.

---
2. Second
1. First
  . Indented

---
// Test automatic numbering in summed content.
#for i in range(5) {
   [. #roman(1 + i)]
}

---
// Test label pattern.
#set enum(label: "~ A:")
. First
. Second

#set enum(label: "(*)")
. A
. B
. C

#set enum(label: "i)")
. A
. B

---
// Test label closure.
#enum(
   start: 4,
   spacing: -3pt,
   label: n => text(fill: (red, green, blue)(mod(n, 3)), [#upper(letter(n))]),
   [Red], [Green], [Blue],
)

---
// Error: 18-20 invalid pattern
#set enum(label: "")

---
// Error: 18-24 invalid pattern
#set enum(label: "(())")

---
// Error: 18-28 expected content, found boolean
#set enum(label: n => false)
. A
