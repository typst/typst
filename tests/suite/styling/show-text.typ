// Test text replacement show rules.

--- show-text-basic ---
// Test classic example.
#set text(font: "Roboto")
#show "Der Spiegel": smallcaps
Die Zeitung Der Spiegel existiert.

--- show-text-regex ---
// Another classic example.
#show "TeX": [T#h(-0.145em)#box(move(dy: 0.233em)[E])#h(-0.135em)X]
#show regex("(Lua)?(La)?TeX"): name => box(text(font: "New Computer Modern")[#name])

TeX, LaTeX, LuaTeX and LuaLaTeX!

--- show-text-cyclic ---
// Test direct cycle.
#show "Hello": text(red)[Hello]
Hello World!

--- show-text-cyclic-raw ---
// Test replacing text with raw text.
#show "rax": `rax`
The register rax.

--- show-text-indirectly-cyclic ---
// Test indirect cycle.
#show "Good": [Typst!]
#show "Typst": [Fun!]
#show "Fun": [Good!]

#set text(ligatures: false)
Good \
Fun \
Typst \

--- show-text-exactly-once ---
// Test that replacements happen exactly once.
#show "A": [BB]
#show "B": [CC]
AA (8)

--- show-text-regex-word-boundary ---
// Test caseless match and word boundaries.
#show regex("(?i)\bworld\b"): [🌍]

Treeworld, the World of worlds, is a world.

--- show-text-empty ---
// Test there is no crashing on empty strings
// Error: 1:7-1:9 text selector is empty
#show "": []

--- show-text-regex-empty ---
// Error: 1:7-1:16 regex selector is empty
#show regex(""): [AA]

--- show-text-regex-matches-empty ---
// Error: 1:7-1:42 regex matches empty text
#show regex("(VAR_GLOBAL|END_VAR||BOOL)") : []

--- show-text-regex-character-class ---
// This is a fun one.
#set par(justify: true)
#show regex("\S"): letter => box(stroke: 1pt, inset: 2pt, upper(letter))
#lorem(5)

--- show-text-regex-case-insensitive ---
// See also: https://github.com/mTvare6/hello-world.rs
#show regex("(?i)rust"): it => [#it (🚀)]
Rust is memory-safe and blazingly fast. Let's rewrite everything in rust.

--- show-text-get-text-on-it ---
// Test accessing the string itself.
#show "hello": it => it.text.split("").map(upper).join("|")
Oh, hello there!

--- show-text-in-other-show ---
// Replace worlds but only in lists.
#show list: it => [
  #show "World": [🌎]
  #it
]

World
- World

--- show-text-path-resolving ---
// Test absolute path in layout phase.

#show "GRAPH": image("/assets/images/graph.png")

The GRAPH has nodes.

--- show-set-text-order-adjacent-1 ---
#show "He": set text(red)
#show "ya": set text(blue)
Heya

--- show-set-text-order-contained-1 ---
#show "Heya": set text(red)
#show   "ya": set text(blue)
Heya

--- show-set-text-order-contained-3 ---
#show "He": set text(red)
#show "Heya": set text(blue)
Heya

--- show-set-text-order-overlapping-1 ---
#show "Heya": set text(red)
#show   "yaho": set text(blue)
Heyaho

--- show-set-text-order-adjacent-2 ---
#show "He": set text(red)
#show "ya": set text(weight: "bold")
Heya

--- show-set-text-order-contained-2 ---
#show "Heya": set text(red)
#show   "ya": set text(weight: "bold")
Heya

--- show-set-text-order-contained-4 ---
#show "He": set text(red)
#show "Heya": set text(weight: "bold")
Heya

--- show-set-text-order-overlapping-2 ---
#show "Heya": set text(red)
#show   "yaho": set text(weight: "bold")
Heyaho
