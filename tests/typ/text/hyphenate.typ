// Test hyphenation.

---
// Test hyphenating english and greek.
#set text(hyphenate: true)
#set page(width: auto)
#grid(
  columns: (50pt, 50pt),
  [Warm welcomes to Typst.],
  text(lang: "el")[διαμερίσματα. \ λατρευτός],
)

---
// Test disabling hyphenation for short passages.
#set page(width: 110pt)
#set text(hyphenate: true)

Welcome to wonderful experiences. \
Welcome to `wonderful` experiences. \
Welcome to #text(hyphenate: false)[wonderful] experiences. \
Welcome to wonde#text(hyphenate: false)[rf]ul experiences. \

// Test enabling hyphenation for short passages.
#set text(hyphenate: false)
Welcome to wonderful experiences. \
Welcome to wo#text(hyphenate: true)[nd]erful experiences. \

---
// Hyphenate between shape runs.
#set page(width: 80pt)
#set text(hyphenate: true)
It's a #emph[Tree]beard.

---
// Test shy hyphens.
#set text(lang: "de", hyphenate: true)
#grid(
  columns: 2 * (20pt,),
  gutter: 20pt,
  [Barankauf],
  [Bar-?ankauf],
)

---
// This sequence would confuse hypher if we passed trailing / leading
// punctuation instead of just the words. So this tests that we don't
// do that. The test passes if there's just one hyphenation between
// "net" and "works".
#set page(width: 60pt)
#set text(hyphenate: true)
#h(6pt) networks, the rest.

---
#set page(height: 60pt)

#let sample = lorem(12)

#sample
#pagebreak()
#set text(prevent-widows-and-orphans: false)
#sample

---
#set par(justify: true)

#let sample = [please avoid runts in this text.]

#sample
#pagebreak()
#set text(runt-cost: 10000%)
#sample

---
#set par(justify: true)
#set text(size: 6pt)

#let sample = [a a a a a a a a a a a a a a a a a a a a a a a a a]

#sample
#pagebreak()
#set text(runt-cost: 0%)
#sample

---
#set par(justify: true)

#let sample = [we've increase the hyphenation cost.]

#sample
#pagebreak()
#set text(hyphenation-cost: 10000%)
#sample
