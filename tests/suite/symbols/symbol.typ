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

--- symbol-constructor-duplicate-modifier ---
// Error: 2:3-2:31 duplicate modifier within variant: "duplicate"
// Hint: 2:3-2:31 the modifiers of a variant constitute a set, meaning repetition does not matter
#symbol(
  ("duplicate.duplicate", "x"),
)

--- symbol-constructor-duplicate-default-variant ---
// Error: 3:3-3:6 duplicate default variant
#symbol(
  "x",
  "y",
)

--- symbol-constructor-duplicate-empty-variant ---
// Error: 3:3-3:12 duplicate default variant
#symbol(
  ("", "x"),
  ("", "y"),
)

--- symbol-constructor-default-and-empty-variants ---
// Error: 3:3-3:12 duplicate default variant
#symbol(
  "x",
  ("", "y"),
)

--- symbol-constructor-duplicate-variant ---
// Error: 3:3-3:29 duplicate variant: "duplicate.variant"
#symbol(
  ("duplicate.variant", "x"),
  ("duplicate.variant", "y"),
)

--- symbol-constructor-duplicate-variant-different-order ---
// Error: 3:3-3:29 duplicate variant: "variant.duplicate"
// Hint: 3:3-3:29 the modifiers of a variant constitute a set, meaning order does not matter
#symbol(
  ("duplicate.variant", "x"),
  ("variant.duplicate", "y"),
)

--- symbol-unknown-modifier ---
// Error: 13-20 unknown symbol modifier
#emoji.face.garbage

--- symbol-repr ---
#test(
  repr(sym.amp),
  `symbol("&", ("inv", "⅋"))`.text,
)
#test(
  repr(sym.amp.inv),
  `symbol("⅋")`.text,
)
#test(
  repr(sym.arrow.double.r),
  ```
  symbol(
    "⇒",
    ("bar", "⤇"),
    ("long", "⟹"),
    ("long.bar", "⟾"),
    ("not", "⇏"),
    ("l", "⇔"),
    ("l.long", "⟺"),
    ("l.not", "⇎"),
  )
  ```.text,
)
#test(repr(sym.smash), "symbol(\"⨳\")")

#let envelope = symbol(
  "🖂",
  ("stamped", "🖃"),
  ("stamped.pen", "🖆"),
  ("lightning", "🖄"),
  ("fly", "🖅"),
)
#test(
  repr(envelope),
  ```
  symbol(
    "🖂",
    ("stamped", "🖃"),
    ("stamped.pen", "🖆"),
    ("lightning", "🖄"),
    ("fly", "🖅"),
  )
  ```.text,
)
#test(
  repr(envelope.stamped),
  `symbol("🖃", ("pen", "🖆"))`.text,
)
#test(
  repr(envelope.stamped.pen),
  `symbol("🖆")`.text,
)
#test(
  repr(envelope.lightning),
  `symbol("🖄")`.text,
)
#test(
  repr(envelope.fly),
  `symbol("🖅")`.text,
)
