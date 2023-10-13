// In this bug, the first line of the second paragraph was on its page alone an
// the rest moved down. The reason was that the second block resulted in
// overlarge frames because the region wasn't finished properly.

---
#set page(height: 70pt)
#block[This file tests a bug where an almost empty page occurs.]
#block[
  The text in this second block was torn apart and split up for
  some reason beyond my knowledge.
]
