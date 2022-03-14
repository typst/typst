// Test hyphenation.

---
// Hyphenate english.
#set page(width: 70pt)
#set par(lang: "en", hyphenate: true)
Warm welcomes to Typst.

---
// Hyphenate greek.
#set page(width: 60pt)
#set par(lang: "el", hyphenate: true)
διαμερίσματα. \
λατρευτός

---
// Hyphenate between shape runs.
#set par(lang: "en", hyphenate: true)
#set page(width: 80pt)

It's a #emph[Tree]beard.

---
// This sequence would confuse hypher if we passed trailing / leading
// punctuation instead of just the words. So this tests that we don't
// do that. The test passes if there's just one hyphenation between
// "net" and "works".
#set page(width: 70pt)
#set par(lang: "en", hyphenate: true)
#h(6pt) networks, the rest.
