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
#set enum(number-align: center + horizon)
1.  #box(fill: teal, inset: 10pt )[a]
8.  #box(fill: teal, inset: 10pt )[b]
16. #box(fill: teal,inset: 10pt )[c]

---
// Number align option should not be affected by the context.
#set align(center)
#set enum(number-align: start)

4.  c
8.  d
16. e\ f
   2.  f\ g
   32. g
   64. h

---
// Test valid number align values (horizontal and vertical)
// Ref: false
#set enum(number-align: start)
#set enum(number-align: end)
#set enum(number-align: left)
#set enum(number-align: center)
#set enum(number-align: right)
#set enum(number-align: top)
#set enum(number-align: horizon)
#set enum(number-align: bottom)

