// Test the `upper` and `lower` functions.
// Ref: false

---
#let memes = "ArE mEmEs gReAt?";
#test(lower(memes), "are memes great?")
#test(upper(memes), "ARE MEMES GREAT?")
#test(upper("Ελλάδα"), "ΕΛΛΆΔΑ")

---
// Error: 8-9 expected string or content, found integer
#upper(1)
