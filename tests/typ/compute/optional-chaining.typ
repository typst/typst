// Test optional chaining.
// Ref: false

---

#let a = "code"
#assert.eq(a?.len(), 4)

#let b = none
#assert.eq(b?.len(), none)
