--- show-where-optional-field-raw ---
// Test that where selectors also trigger on set rule fields.
#show raw.where(block: false): box.with(
  fill: luma(220),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)

This is #raw("fn main() {}") some text.

--- show-where-optional-field-text ---
// Note: This show rule is horribly inefficient because it triggers for
// every individual text element. But it should still work.
#show text.where(lang: "de"): set text(red)

#set text(lang: "es")
Hola, mundo!

#set text(lang: "de")
Hallo Welt!

#set text(lang: "en")
Hello World!

--- show-where-folding-text-size ---
// Test that folding is taken into account.
#set text(5pt)
#set text(2em)

#[
  #show text.where(size: 2em): set text(blue)
  2em not blue
]

#[
  #show text.where(size: 10pt): set text(blue)
  10pt blue
]

--- show-where-folding-stroke ---
// Test again that folding is taken into account.
#set rect(width: 40pt, height: 10pt)
#set rect(stroke: blue)
#set rect(stroke: 2pt)

#{
  show rect.where(stroke: blue): "Not Triggered"
  rect()
}
#{
  show rect.where(stroke: 2pt): "Not Triggered"
  rect()
}
#{
  show rect.where(stroke: 2pt + blue): "Triggered"
  rect()
}

--- show-where-resolving-length ---
// Test that resolving is *not* taken into account.
#set line(start: (1em, 1em + 2pt))

#{
  show line.where(start: (1em, 1em + 2pt)): "Triggered"
  line()
}
#{
  show line.where(start: (10pt, 12pt)): "Not Triggered"
  line()
}


--- show-where-resolving-hyphenate ---
// Test again that resolving is *not* taken into account.
#set text(hyphenate: auto)

#[
  #show text.where(hyphenate: auto): underline
  Auto
]
#[
  #show text.where(hyphenate: true): underline
  True
]
#[
  #show text.where(hyphenate: false): underline
  False
]

--- show-where-ty-check ---
#show link.where(dest: str): set text(blue)

= Hello <hello>
#link(<hello>)[Label] \
#link((page: 1, x: 0pt, y: 0pt))[Position] \
#link("https://typst.app")[String]

--- show-where-ty-itself ---
// There is some ambiguity: If we pass a type, we could also have wanted to
// match a field whose value is exactly that type rather than an instance of it.
// For now, this should be an exceedingly rare requirement and we don't provide
// a way to do it. In the future, when we unify types and elements, and
// selectors and type hints, we'll probably provide a selector/pattern that
// supports this; something like `pattern.literal(int)`. Then, the function
// `let f(x: int) = ..` only allows integers, but
// `let f(x: pattern.literal(int)) = ..` only allows the int type itself.
//
// For what it's worth, similar ambiguities exist elsewhere
// - for `show: f` where we might want to literally displayed `f` instead of
//   using it as a show rule recipe
// - for `array.contains` where we could also want to search for a function
//   rather than using it to filter
// - for `array.find` where `none` can mean no match or we found a `none`
//
// This is just hard to avoid in dynamically typed languages ...

#show metadata: [no int]
#show metadata.where(value: int): [int]

#metadata(0) \
#metadata(int) \
#metadata(false) \
#metadata(bool)
