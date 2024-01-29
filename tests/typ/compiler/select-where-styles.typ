// Test that where selectors also work with settable fields.

---
// Test that where selectors also trigger on set rule fields.
#show raw.where(block: false): box.with(
  fill: luma(220),
  inset: (x: 3pt, y: 0pt),
  outset: (y: 3pt),
  radius: 2pt,
)

This is #raw("fn main() {}") some text.

---
// Note: This show rule is horribly inefficient because it triggers for
// every individual text element. But it should still work.
#show text.where(lang: "de"): set text(red)

#set text(lang: "es")
Hola, mundo!

#set text(lang: "de")
Hallo Welt!

#set text(lang: "en")
Hello World!

---
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

---
// Test again that folding is taken into account.
#set rect(width: 40pt, height: 10pt)
#set rect(stroke: blue)
#set rect(stroke: 2pt)

#{
  show rect.where(stroke: blue): "Trigger"
  rect()
}
#{
  show rect.where(stroke: 2pt): "Trigger"
  rect()
}
#{
  show rect.where(stroke: 2pt + blue): "Trigger"
  rect()
}

---
// Test that resolving is taken into account.
#set line(start: (1em, 1em + 2pt))

#{
  show line.where(start: (1em, 1em + 2pt)): "Trigger"
  line()
}
#{
  show line.where(start: (10pt, 12pt)): "Trigger"
  line()
}
