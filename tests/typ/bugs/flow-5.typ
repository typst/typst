// This bug caused an index-out-of-bounds panic when layouting paragraphs needed
// multiple reorderings.

---
#set page(height: 200pt)
#lorem(30)

#figure(placement: auto, block(height: 100%))

#lorem(10)

#lorem(10)

