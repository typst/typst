// Test text replacement show rules.

---
// Test classic example.
#set text("Roboto")
#show phrase: "Der Spiegel" as smallcaps[#phrase]
Die Zeitung Der Spiegel existiert.

---
// Another classic example.
#show "TeX" as [T#h(-0.145em)#move(dy: 0.233em)[E]#h(-0.135em)X]
#show name: regex("(Lua)?(La)?TeX") as box(text("Latin Modern Roman")[#name])

TeX, LaTeX, LuaTeX and LuaLaTeX!

---
// Test out-of-order guarding.
#show "Good" as [Typst!]
#show "Typst" as [Fun!]
#show "Fun" as [Good!]
#show enum as []

Good \
Fun \
Typst \

---
// Test that replacements happen exactly once.
#show "A" as [BB]
#show "B" as [CC]
AA (8)

---
// Test caseless match and word boundaries.
#show regex("(?i)\bworld\b") as [🌍]

Treeworld, the World of worlds, is a world.

---
// This is a fun one.
#set par(justify: true)
#show letter: regex("\S") as rect(inset: 2pt)[#upper(letter)]
#lorem(5)

---
// See also: https://github.com/mTvare6/hello-world.rs
#show it: regex("(?i)rust") as [#it (🚀)]
Rust is memory-safe and blazingly fast. Let's rewrite everything in rust.

---
// Replace worlds but only in lists.
#show node: list as [
  #show "World" as [🌎]
  #node
]

World
- World
