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

--- lorem-missing-words eval ---
// Error: 2-9 missing argument: words
#lorem()

--- lorem-word-count eval ---
// https://github.com/typst/typst/issues/6186
#let count(n) = lorem(n).replace("–", "").replace(".", "").split(" ").filter(s => s != "").len()
#test(count(193), 193)
#test(count(194), 194)
#test(count(195), 195)

