// Test hyphenation.

--- hyphenate ---
// Test hyphenating english and greek.
#set text(hyphenate: true)
#set page(width: auto)
#grid(
  columns: (50pt, 50pt),
  [Warm welcomes to Typst.],
  text(lang: "el")[διαμερίσματα. \ λατρευτός],
)

--- hyphenate-off-temporarily ---
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

--- hyphenate-between-shape-runs ---
// Hyphenate between shape runs.
#set page(width: 80pt)
#set text(hyphenate: true)
It's a #emph[Tree]beard.

--- hyphenate-shy ---
// Test shy hyphens.
#set text(lang: "de", hyphenate: true)
#grid(
  columns: 2 * (20pt,),
  gutter: 20pt,
  [Barankauf],
  [Bar-?ankauf],
)

--- hyphenate-punctuation ---
// This sequence would confuse hypher if we passed trailing / leading
// punctuation instead of just the words. So this tests that we don't
// do that. The test passes if there's just one hyphenation between
// "net" and "works".
#set page(width: 60pt)
#set text(hyphenate: true)
#h(6pt) networks, the rest.

--- costs-widow-orphan ---
#set page(height: 60pt)

#let sample = lorem(12)

#sample
#pagebreak()
#set text(costs: (widow: 0%, orphan: 0%))
#sample

--- costs-runt-avoid ---
#set par(justify: true)

#let sample = [please avoid runts in this text.]

#sample
#pagebreak()
#set text(costs: (runt: 10000%))
#sample

--- costs-runt-allow ---
#set par(justify: true)
#set text(size: 6pt)

#let sample = [a a a a a a a a a a a a a a a a a a a a a a a a a]

#sample
#pagebreak()
#set text(costs: (runt: 0%))
#sample

--- costs-hyphenation-avoid ---
#set par(justify: true)

#let sample = [we've increased the hyphenation cost.]

#sample
#pagebreak()
#set text(costs: (hyphenation: 10000%))
#sample

--- costs-invalid-type ---
// Error: 18-37 expected ratio, found auto
#set text(costs: (hyphenation: auto))

--- costs-invalid-key ---
// Error: 18-52 unexpected key "invalid-key", valid keys are "hyphenation", "runt", "widow", and "orphan"
#set text(costs: (hyphenation: 1%, invalid-key: 3%))

--- costs-access ---
#set text(costs: (hyphenation: 1%, runt: 2%))
#set text(costs: (widow: 3%))
#context {
  assert.eq(text.costs, (hyphenation: 1%, runt: 2%, widow: 3%, orphan: 100%))
}
