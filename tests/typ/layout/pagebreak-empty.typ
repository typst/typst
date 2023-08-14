// Test page breaks on basically empty pages.

---
// After strong place
// Should result in three pages.
First
#pagebreak(weak: true)
#place(right, weak: false)[Strong A]
#pagebreak(weak: true)
Third

---
// After only ignorables
// Should result in two pages.
First
#pagebreak(weak: true)
#counter(page).update(1)
#place(right, weak: true)[Weak A]
#pagebreak(weak: true)
Second

---
// After only ignorables, but regular break
// Should result in three pages.
First
#pagebreak()
#counter(page).update(1)
#place(right, weak: true)[Weak A]
#pagebreak()
Third
