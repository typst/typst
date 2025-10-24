--- show-where-optional-field-raw render ---
// Test that where selectors also trigger on set rule fields.
#show raw.where(block: false): box.with(
  fill: luma(220),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)

This is #raw("fn main() {}") some text.

--- show-where-optional-field-text render ---
// Note: This show rule is horribly inefficient because it triggers for
// every individual text element. But it should still work.
#show text.where(lang: "de"): set text(red)

#set text(lang: "es")
Hola, mundo!

#set text(lang: "de")
Hallo Welt!

#set text(lang: "en")
Hello World!

--- show-where-folding-text-size render ---
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

--- show-where-folding-stroke render ---
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

--- show-where-resolving-length render ---
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


--- show-where-resolving-hyphenate render ---
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
