// Test text decorations.

---
#let red = rgb("fc0030")

// Basic strikethrough.
#strike[Statements dreamt up by the utterly deranged.]

// Move underline down.
#underline(offset: 5pt)[Further below.]

// Different color.
#underline(red, evade: false)[Critical information is conveyed here.]

// Inherits font color.
#text(fill: red, underline[Change with the wind.])

// Both over- and underline.
#overline(underline[Running amongst the wolves.])

---
#let redact = strike.with(10pt, extent: 5%)
#let highlight = strike.with(
  stroke: rgb("abcdef88"),
  thickness: 10pt,
  extent: 5%,
)

// Abuse thickness and transparency for redacting and highlighting stuff.
Sometimes, we work #redact[in secret].
There might be #highlight[redacted] things.
