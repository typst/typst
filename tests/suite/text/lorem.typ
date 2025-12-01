// Test blind text.

--- lorem paged ---
// Test basic call.
#lorem(19)

--- lorem-pars paged ---
// Test custom paragraphs with user code.
#set text(8pt)

#{
  let sentences = lorem(59)
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

--- lorem-missing-words paged ---
// Error: 2-9 missing argument: words
#lorem()
