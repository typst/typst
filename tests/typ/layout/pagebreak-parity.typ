// Test clearing to even or odd pages.

---
#set page(width: 80pt, height: 30pt)
First
#pagebreak(to: "odd")
Third
#pagebreak(to: "even")
Fourth
#pagebreak(to: "even")
Sixth
#pagebreak()
Seventh
#pagebreak(to: "odd")
#page[Ninth]

---
#set page(width: auto, height: auto)

// Test with auto-sized page.
First
#pagebreak(to: "odd")
Third

---
#set page(height: 30pt, width: 80pt)

// Test when content extends to more than one page
First

Second

#pagebreak(to: "odd")

Third

---
// Test headers and footers are skipped on pagebreak-ed empty pages.
#set page(
  width: 80pt, height: 30pt, header: [header], footer: [
    #counter(page).display("1 of 1", both: true)
  ]
)

First
#pagebreak(to: "odd")
Third
