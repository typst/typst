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
// Auto align should inherit horizontal alignment from context
#set enum(number-align: auto)
1.  a
10. b\ c\ d
   2.  c\ d\ e
   12. d

---
// Auto align should inherit horizontal alignment from context
#set align(center + horizon)
#set enum(number-align: auto)
1.  a
10. b\ c\ d
100. e\ e
    3.  c\ d\ e
    13. d
    133. e\ e
