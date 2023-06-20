// Test optional chaining.
// Ref: false

---

#let a = (iaj-zav: "xbsio la")
#assert.eq(a?.iaj-zav, "xbsio la")

#let b = none
#assert.eq(b?.iaj-zav, none)

---

// Test that the optional field access is evaluated.
#let a = (iaj-zav: "xbsio la")

// Error: 11-34 equality assertion failed: value "xbsio la" was not equal to "voepfxo"
#assert.eq(a?.iaj-zav, "voepfxo")

---

#let a = "code"
#assert.eq(a?.len(), 4)

#let b = none
#assert.eq(b?.len(), none)

---

// Test that the optional field access is evaluated.
#let a = "code"

// Error: 11-25 equality assertion failed: value 4 was not equal to 10
#assert.eq(a?.len(), 10)

---

// Test that non-optional field access on a none value results in an error.

#let b = none

// Error: 14-21 cannot access fields on type none
#assert.eq(b.iaj-zav, none)

---

// Test that non-optional method calls on a none value results in an error.

#let b = none

// Error: 12-19 type none has no method `len`
#assert.eq(b.len(), none)
