// Checks that scoped element such as `raw.line` cannot be constructed
// and therefore do not cause a compiler crash.
// Ref: false

---

// Hint: 10-12 use the `raw` element instead
// Error: 10-12 cannot construct a `raw.line` element
#raw.line()

---

// Hint: 16-18 use the `figure` element instead
// Error: 16-18 cannot construct a `figure.caption` element
#figure.caption()

---

// Hint: 16-18 use the `footnote` element instead
// Error: 16-18 cannot construct a `footnote.entry` element
#footnote.entry()

---

// Hint: 15-17 use the `outline` element instead
// Error: 15-17 cannot construct an `outline.entry` element
#outline.entry()
