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

--- hyphenate-outside-of-words ---
// More tests for hyphenation of non-words.
#set text(hyphenate: true)
#block(width: 0pt, "doesn't")
#block(width: 0pt, "(OneNote)")
#block(width: 0pt, "(present)")

#set text(lang: "de")
#block(width: 0pt, "(bzw.)")

--- hyphenate-pt-repeat-hyphen-natural-word-breaking ---
// The word breaker naturally breaks arco-da-velha at arco-/-da-velha,
// so we shall repeat the hyphen, even that hyphenate is set to false.
#set page(width: 4cm)
#set text(lang: "pt")

Alguma coisa no arco-da-velha é algo que está muito longe.

--- hyphenate-pt-repeat-hyphen-hyphenate-true ---
#set page(width: 4cm)
#set text(lang: "pt", hyphenate: true)

Alguma coisa no arco-da-velha é algo que está muito longe.

--- hyphenate-pt-repeat-hyphen-hyphenate-true-with-emphasis ---
#set page(width: 4cm)
#set text(lang: "pt", hyphenate: true)

Alguma coisa no _arco-da-velha_ é algo que está muito longe.

--- hyphenate-pt-no-repeat-hyphen ---
#set page(width: 4cm)
#set text(lang: "pt", hyphenate: true)

Um médico otorrinolaringologista cuida da garganta do paciente.

--- hyphenate-pt-dash-emphasis ---
// If the hyphen is followed by a space we shall not repeat the hyphen
// at the next line
#set page(width: 4cm)
#set text(lang: "pt", hyphenate: true)

Quebabe é a -melhor- comida que existe.

--- hyphenate-es-repeat-hyphen ---
#set page(width: 6cm)
#set text(lang: "es", hyphenate: true)

Lo que entendemos por nivel léxico-semántico, en cuanto su sentido más
gramatical: es aquel que estudia el origen y forma de las palabras de
un idioma.

--- hyphenate-es-captalized-names ---
// If the hyphen is followed by a capitalized word we shall not repeat
//  the hyphen at the next line
#set page(width: 6.2cm)
#set text(lang: "es", hyphenate: true)

Tras el estallido de la contienda Ruiz-Giménez fue detenido junto a sus
dos hermanos y puesto bajo custodia por las autoridades republicanas, con
el objetivo de protegerle de las patrullas de milicianos.

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
#context test(text.costs, (hyphenation: 1%, runt: 2%, widow: 3%, orphan: 100%))
