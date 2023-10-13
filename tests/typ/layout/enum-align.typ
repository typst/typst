// Test the alignment of enum numbers.

---
// Alignment shouldn't affect number
#set align(horizon)

+ ABCDEF\ GHIJKL\ MNOPQR
   + INNER\ INNER\ INNER
+ BACK\ HERE

---
// Enum number alignment should be 'end' by default
1. a
10. b
100. c

#set enum(number-align: start)
1.  a
8.  b
16. c

---
// Number align option should not be affected by the context
#set align(center)
#set enum(number-align: start)

4.  c
8.  d
16. e\ f
   2.  f\ g
   32. g
   64. h

---
// Test valid number align values (horizontal)
// Ref: false
#set enum(number-align: start)
#set enum(number-align: end)
#set enum(number-align: left)
#set enum(number-align: right)

---
// Error: 25-28 expected `start`, `left`, `center`, `right`, or `end`, found top
#set enum(number-align: top)
