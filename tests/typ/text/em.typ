// Test font-relative sizing.

---
#set text(size: 5pt)
A // 5pt
[
  #set text(size: 200%)
  B // 10pt
  [
    #set text(size: 150% + 1pt)
    C // 16pt
    #text(size: 200%)[D] // 32pt
    E // 16pt
  ]
  F // 10pt
]
G // 5pt
