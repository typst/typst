// Test the `repeat` function.

--- repeat-basic ---
// Test multiple repeats.
#let sections = (
  ("Introduction", 1),
  ("Approach", 1),
  ("Evaluation", 3),
  ("Discussion", 15),
  ("Related Work", 16),
  ("Conclusion", 253),
)

#for section in sections [
  #section.at(0) #box(width: 1fr, repeat[.]) #section.at(1) \
]

--- repeat-dots-rtl ---
// Test dots with RTL.
#set text(lang: "ar")
مقدمة #box(width: 1fr, repeat[.]) 15

--- repeat-empty ---
// Test empty repeat.
A #box(width: 1fr, repeat[]) B

--- repeat-unboxed ---
// Test unboxed repeat.
#repeat(rect(width: 2em, height: 1em))

--- repeat-align-and-dir ---
// Test single repeat in both directions.
A#box(width: 1fr, repeat(rect(width: 6em, height: 0.7em)))B

#set align(center)
A#box(width: 1fr, repeat(rect(width: 6em, height: 0.7em)))B

#set text(dir: rtl)
ريجين#box(width: 1fr, repeat(rect(width: 4em, height: 0.7em)))سون

--- repeat-unrestricted ---
// Error: 2:2-2:13 repeat with no size restrictions
#set page(width: auto)
#repeat(".")
