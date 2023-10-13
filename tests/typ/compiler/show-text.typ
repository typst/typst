// Test text replacement show rules.

---
// Test classic example.
#set text(font: "Roboto")
#show "Der Spiegel": smallcaps
Die Zeitung Der Spiegel existiert.

---
// Another classic example.
#show "TeX": [T#h(-0.145em)#box(move(dy: 0.233em)[E])#h(-0.135em)X]
#show regex("(Lua)?(La)?TeX"): name => box(text(font: "New Computer Modern")[#name])

TeX, LaTeX, LuaTeX and LuaLaTeX!

---
// Test that replacements happen exactly once.
#show "A": [BB]
#show "B": [CC]
AA (8)

---
// Test caseless match and word boundaries.
#show regex("(?i)\bworld\b"): [🌍]

Treeworld, the World of worlds, is a world.

---
// Test there is no crashing on empty strings
// Error: 1:7-1:9 text selector is empty
#show "": []

---
// Error: 1:7-1:16 regex selector is empty
#show regex(""): [AA]

---
// Error: 1:7-1:42 regex matches empty text
#show regex("(VAR_GLOBAL|END_VAR||BOOL)") : []

---
// This is a fun one.
#set par(justify: true)
#show regex("\S"): letter => box(stroke: 1pt, inset: 2pt, upper(letter))
#lorem(5)

---
// See also: https://github.com/mTvare6/hello-world.rs
#show regex("(?i)rust"): it => [#it (🚀)]
Rust is memory-safe and blazingly fast. Let's rewrite everything in rust.

---
// Test accessing the string itself.
#show "hello": it => it.text.split("").map(upper).join("|")
Oh, hello there!

---
// Replace worlds but only in lists.
#show list: it => [
  #show "World": [🌎]
  #it
]

World
- World

---
// Test absolute path in layout phase.

#show "GRAPH": image("/files/graph.png")

The GRAPH has nodes.
