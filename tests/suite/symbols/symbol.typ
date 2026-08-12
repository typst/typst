// Test symbols.

--- symbol paged ---
#emoji.face
#emoji.woman.old
#emoji.turtle

#set text(font: "New Computer Modern Math")
#sym.arrow
#sym.arrow.l
#sym.arrow.r.squiggly
#sym.arrow.tr.hook

#sym.arrow.r;this and this#sym.arrow.l;

--- symbol-constructor paged ---
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

--- symbol-constructor-empty eval ---
// Error: 2-10 expected at least one variant
#symbol()

--- symbol-constructor-invalid-modifier eval ---
#symbol(
// Error: 3-24 invalid symbol modifier: " id!"
  ("invalid. id!", "x")
)

--- symbol-constructor-duplicate-modifier eval ---
#symbol(
  // Error: 3-31 duplicate modifier within variant: "duplicate"
  // Hint: 3-31 modifiers are not ordered, so each one may appear only once
  ("duplicate.duplicate", "x"),
)

--- symbol-constructor-duplicate-default-variant eval ---
#symbol(
  "x",
  // Error: 3-6 duplicate default variant
  "y",
)

--- symbol-constructor-duplicate-empty-variant eval ---
#symbol(
  ("", "x"),
  // Error: 3-12 duplicate default variant
  ("", "y"),
)

--- symbol-constructor-default-and-empty-variants eval ---
#symbol(
  "x",
  // Error: 3-12 duplicate default variant
  ("", "y"),
)

--- symbol-constructor-duplicate-variant eval ---
#symbol(
  ("duplicate.variant", "x"),
  // Error: 3-29 duplicate variant: "duplicate.variant"
  ("duplicate.variant", "y"),
)

--- symbol-constructor-duplicate-variant-different-order eval ---
#symbol(
  ("duplicate.variant", "x"),
  // Error: 3-29 duplicate variant: "variant.duplicate"
  // Hint: 3-29 variants with the same modifiers are identical, regardless of their order
  ("variant.duplicate", "y"),
)

--- symbol-constructor-empty-variant-value eval ---
#symbol(
  // Error: 3-5 invalid variant value: ""
  // Hint: 3-5 variant value must be exactly one grapheme cluster
  "",
  // Error: 3-16 invalid variant value: ""
  // Hint: 3-16 variant value must be exactly one grapheme cluster
  ("empty", "")
)

--- symbol-constructor-multi-cluster-variant-value eval ---
#symbol(
  // Error: 3-7 invalid variant value: "aa"
  // Hint: 3-7 variant value must be exactly one grapheme cluster
  "aa",
  // Error: 3-14 invalid variant value: "bb"
  // Hint: 3-14 variant value must be exactly one grapheme cluster
  ("b", "bb")
)

--- symbol-unknown-modifier eval ---
// Error: 13-20 unknown symbol modifier
#emoji.face.garbage

--- symbol-repr eval ---
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

--- issue-5930-symbol-label paged ---
#emoji.face<lab>
#context test(query(<lab>).first().text, "😀")

--- presentation-selectors paged ---
// Currently, presentation selectors do not cause font fallback when the main
// font supports at least one presentation, instead causing a fallback of the
// presentation form. This should probably be solved at some point, making the
// emojis below render with an emoji form.
// See: https://github.com/typst/typst/pull/6875.
#sym.copyright #emoji.copyright \
#sym.suit.heart #emoji.suit.heart

--- symbol-deprecation-warnings eval ---
// Test symbol deprecation warnings from codex.
#let pairs = (("emoji", emoji), ("sym", sym))
#let symbol-paths = ()

#while pairs.len() > 0 {
  let (path, sym-or-mod) = pairs.pop()
  let path-dot = path + "."

  if type(sym-or-mod) != symbol {
    pairs += dictionary(sym-or-mod)
      .pairs()
      .map(((name, s)) => (path-dot + name, s))
    continue
  }

  let mod-array = repr(sym-or-mod)
    .trim("symbol")
    // Make sure the closing paren has a comma to produce an array.
    .replace(regex("(,\\n)?\\)$"), ",)")

  let (maybe-mod-pair, ..modifier-pairs) = eval(mod-array)

  // The modifiers can start with a default symbol or a modifier pair.
  if type(maybe-mod-pair) == str {
    symbol-paths.push(path)
  } else {
    symbol-paths.push(path-dot + maybe-mod-pair.first())
  }
  // The rest are all modifier pairs.
  symbol-paths += modifier-pairs.map(((mod, _)) => path-dot + mod)
}

// Warning: 27-31 `tack.double` is deprecated, use repeated letters instead, e.g., `tack.rr`
// Warning: 27-31 `tack.double` is deprecated, use repeated letters instead, e.g., `tack.ll`
// Warning: 27-31 `tack.double` is deprecated, use repeated letters instead, e.g., `tack.tt`
// Warning: 27-31 `tack.double` is deprecated, use repeated letters instead, e.g., `tack.bb`
// Warning: 27-31 `lt.tri` is deprecated, use `lt.closed` instead
// Warning: 27-31 `join` is deprecated, use `bowtie.big` instead
// Warning: 27-31 `gt.tri` is deprecated, use `gt.closed` instead
#let _ = symbol-paths.map(eval)
// This should be updated whenever the set of deprecated symbols updates.
