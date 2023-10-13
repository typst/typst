// Test tracking characters apart or together.

---
// Test tracking.
#set text(tracking: -0.01em)
I saw Zoe yӛsterday, on the tram.

---
// Test tracking for only part of paragraph.
I'm in#text(tracking: 0.15em + 1.5pt)[ spaace]!

---
// Test that tracking doesn't disrupt mark placement.
#set text(font: ("PT Sans", "Noto Serif Hebrew"))
#set text(tracking: 0.3em)
טֶקסט

---
// Test tracking in arabic text (makes no sense whatsoever)
#set text(tracking: 0.3em)
النص

---
// Test word spacing.
#set text(spacing: 1em)
My text has spaces.

---
// Test word spacing relative to the font's space width.
#set text(spacing: 50% + 1pt)
This is tight.
