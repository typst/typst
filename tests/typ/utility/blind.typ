// Test blind text.

---
// Test basic call.
#lipsum(19)

---
// Test custom paragraphs with user code.
#set text(8pt)

{
  let sentences = lipsum(59)
    .split(".")
    .filter(s => s != "")
    .map(s => s + ".")

  let used = 0
  for s in sentences {
    if used < 2 {
      used += 1
    } else {
      parbreak()
      used = 0
    }
    s.trim()
    [ ]
  }
}

---
// Error: 8-10 missing argument: number of words
#lipsum()
