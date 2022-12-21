// Test fr units in stacks.

---
#set page(height: 3.5cm)
#stack(
  dir: ltr,
  spacing: 1fr,
  ..for c in "ABCDEFGHI" {([#c],)}
)

Hello
#v(2fr)
from #h(1fr) the #h(1fr) wonderful
#v(1fr)
World! 🌍

---
#set page(height: 2cm)
#set text(white)
#rect(fill: forest)[
          #v(1fr)
  #h(1fr) Hi you!
]
