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
#let one = symbol(
  "1",
  ("emoji", "1️")
)

#envelope
#envelope.stamped
#envelope.pen
#envelope.stamped.pen
#envelope.lightning
#envelope.fly
#one
#one.emoji

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
// Hint: 2:3-2:31 modifiers are not ordered, so each one may appear only once
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
// Hint: 3:3-3:29 variants with the same modifiers are identical, regardless of their order
#symbol(
  ("duplicate.variant", "x"),
  ("variant.duplicate", "y"),
)

--- symbol-constructor-empty-variant-value ---
// Error: 2:3-2:5 invalid variant value: ""
// Hint: 2:3-2:5 variant value must be exactly one grapheme cluster
// Error: 3:3-3:16 invalid variant value: ""
// Hint: 3:3-3:16 variant value must be exactly one grapheme cluster
#symbol(
  "",
  ("empty", "")
)

--- symbol-constructor-multi-cluster-variant-value ---
// Error: 2:3-2:7 invalid variant value: "aa"
// Hint: 2:3-2:7 variant value must be exactly one grapheme cluster
// Error: 3:3-3:14 invalid variant value: "bb"
// Hint: 3:3-3:14 variant value must be exactly one grapheme cluster
#symbol(
  "aa",
  ("b", "bb")
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
    ("struck", "⤃"),
    ("l", "⇔"),
    ("l.long", "⟺"),
    ("l.not", "⇎"),
    ("l.struck", "⤄"),
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

--- symbol-sect-deprecated ---
// Warning: 5-9 `sect` is deprecated, use `inter` instead
$ A sect B = A inter B $

--- issue-5930-symbol-label ---
#emoji.face<lab>
#context test(query(<lab>).first().text, "😀")

--- presentation-selectors ---
// Currently, presentation selectors do not cause a font fallback when the main
// fot supports at least one presentation, instead causing a fallback of the
// presentation form. This should probably be solved at some point, making the
// emojis below render with an emoji form.
// See: https://github.com/typst/typst/pull/6875.
#sym.copyright #emoji.copyright \
#sym.suit.heart #emoji.suit.heart
