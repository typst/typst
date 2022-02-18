// Test wrap.

---
#set page(height: 130pt)
#set text(70%)

#align(center)[
  #text(130%)[*Essay on typography*] \
  T. Ypst
]

#wrap body in columns(2, body)
Great typography is at the essence of great storytelling. It is the medium that
transports meaning from parchment to reader, the wave that sparks a flame
in booklovers and the great fulfiller of human need.

---
// Test wrap in template.
A [_B #wrap c in [*#c*]; C_] D

---
// Test wrap style precedence.
#set text(fill: eastern, size: 150%)
#wrap body in text(fill: forest, body)
Forest

---
// Error: 6-17 set, show and wrap are only allowed directly in markup
{1 + wrap x in y}
