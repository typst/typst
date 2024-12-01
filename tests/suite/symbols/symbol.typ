// Test symbols.

--- symbol ---
#emoji.face
#emoji.woman.old
#emoji.turtle

#set text(font: "New Computer Modern Math")
#sym.arrow
#sym.arrow.l
#sym.arrow.r.squiggly
#sym.arrow.tr.hook

#sym.arrow.r;this and this#sym.arrow.l;

--- symbol-constructor ---
#let envelope = symbol(
  "🖂",
  ("stamped", "🖃"),
  ("stamped.pen", "🖆"),
  ("lightning", "🖄"),
  ("fly", "🖅"),
)

#envelope
#envelope.stamped
#envelope.pen
#envelope.stamped.pen
#envelope.lightning
#envelope.fly

--- symbol-constructor-empty ---
// Error: 2-10 expected at least one variant
#symbol()

--- symbol-constructor-invalid-modifier ---
// Error: 2:3-2:24 invalid symbol modifier: " id!"
#symbol(
  ("invalid. id!", "x")
)

--- symbol-constructor-duplicate-variant ---
// Error: 3:3-3:29 duplicate variant
#symbol(
  ("duplicate.variant", "x"),
  ("duplicate.variant", "y"),
)

--- symbol-unknown-modifier ---
// Error: 13-20 unknown symbol modifier
#emoji.face.garbage

--- symbol-repr ---
#repr(sym.amp) \
#repr(sym.amp.inv) \
#repr(sym.smash)
