// Test the `repeat` function.

---
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
  {section.at(0)} #repeat[.] {section.at(1)} \
]

---
// Test dots with RTL.
#set text(lang: "ar")
مقدمة #repeat[.] 15

---
// Test empty repeat.
A #repeat[] B

---
// Test spaceless repeat.
A#repeat(rect(width: 2.5em, height: 1em))B

---
// Test single repeat in both directions.
A#repeat(rect(width: 6em, height: 0.7em))B

#set align(center)
A#repeat(rect(width: 6em, height: 0.7em))B

#set text(dir: rtl)
ريجين#repeat(rect(width: 4em, height: 0.7em))سون
