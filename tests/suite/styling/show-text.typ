// Test text replacement show rules.

--- show-text-basic render ---
// Test classic example.
#set text(font: "Roboto")
#show "Der Spiegel": smallcaps
Die Zeitung Der Spiegel existiert.

--- show-text-regex render ---
// Another classic example.
#show "TeX": [T#h(-0.145em)#box(move(dy: 0.233em)[E])#h(-0.135em)X]
#show regex("(Lua)?(La)?TeX"): name => box(text(font: "New Computer Modern")[#name])

TeX, LaTeX, LuaTeX and LuaLaTeX!

--- show-text-cyclic render ---
// Test direct cycle.
#show "Hello": text(red)[Hello]
Hello World!

--- show-text-cyclic-raw render ---
// Test replacing text with raw text.
#show "rax": `rax`
The register rax.

--- show-text-indirectly-cyclic render ---
// Test indirect cycle.
#show "Good": [Typst!]
#show "Typst": [Fun!]
#show "Fun": [Good!]

#set text(ligatures: false)
Good \
Fun \
Typst \

--- show-text-exactly-once render ---
// Test that replacements happen exactly once.
#show "A": [BB]
#show "B": [CC]
AA (8)

--- show-text-regex-word-boundary render ---
// Test caseless match and word boundaries.
#show regex("(?i)\bworld\b"): [🌍]

Treeworld, the World of worlds, is a world.

--- show-text-empty render ---
// Test there is no crashing on empty strings
// Error: 1:7-1:9 text selector is empty
#show "": []

--- show-text-regex-empty render ---
// Error: 1:7-1:16 regex selector is empty
#show regex(""): [AA]

--- show-text-regex-matches-empty render ---
// Error: 1:7-1:42 regex matches empty text
#show regex("(VAR_GLOBAL|END_VAR||BOOL)") : []

--- show-text-regex-character-class render ---
// This is a fun one.
#set par(justify: true)
#show regex("\S"): letter => box(stroke: 1pt, inset: 2pt, upper(letter))
#lorem(5)

--- show-text-regex-case-insensitive render ---
// See also: https://github.com/mTvare6/hello-world.rs
#show regex("(?i)rust"): it => [#it (🚀)]
Rust is memory-safe and blazingly fast. Let's rewrite everything in rust.

--- show-text-get-text-on-it render ---
// Test accessing the string itself.
#show "hello": it => it.text.split("").map(upper).join("|")
Oh, hello there!

--- show-text-in-other-show render ---
// Replace worlds but only in lists.
#show list: it => [
  #show "World": [🌎]
  #it
]

World
- World

--- show-text-path-resolving render ---
// Test absolute path in layout phase.

#show "GRAPH": image("/assets/images/graph.png")

The GRAPH has nodes.

--- show-set-text-order-adjacent-1 render ---
#show "He": set text(red)
#show "ya": set text(blue)
Heya

--- show-set-text-order-contained-1 render ---
#show "Heya": set text(red)
#show   "ya": set text(blue)
Heya

--- show-set-text-order-contained-3 render ---
#show "He": set text(red)
#show "Heya": set text(blue)
Heya

--- show-set-text-order-overlapping-1 render ---
#show "Heya": set text(red)
#show   "yaho": set text(blue)
Heyaho

--- show-set-text-order-adjacent-2 render ---
#show "He": set text(red)
#show "ya": set text(weight: "bold")
Heya

--- show-set-text-order-contained-2 render ---
#show "Heya": set text(red)
#show   "ya": set text(weight: "bold")
Heya

--- show-set-text-order-contained-4 render ---
#show "He": set text(red)
#show "Heya": set text(weight: "bold")
Heya

--- show-set-text-order-overlapping-2 render ---
#show "Heya": set text(red)
#show   "yaho": set text(weight: "bold")
Heyaho

--- show-text-smartquote render ---
#show "up,\" she": set text(red)
"What's up," she asked.

--- show-text-apostrophe render ---
#show regex("Who's|We've"): highlight
Who's got it? \
We've got it.

--- show-text-citation render ---
#show "hey": [@arrgh]
@netwok hey

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- show-text-list render ---
#show "hi": [- B]
- A
hi
- C

--- show-text-citation-smartquote render ---
#show "hey \"": [@arrgh]
#show "dis": [@distress]
@netwok hey " dis

#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "american-physics-society")

--- show-text-in-citation render ---
#show "A": "B"
#show "[": "("
#show "]": ")"
#show "[2]": set text(red)

@netwok A \
@arrgh B

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- show-text-linebreak render ---
#show "lo\nwo": set text(red)
Hello #[ ] \
#[ ] #[ ] world!

--- show-text-line-wrapping render ---
#show "start end": "word"
start
end

--- show-text-after-normal-show render ---
#show rect: "world"
#show "lo wo": set text(red)
hello #rect()

--- show-text-space-collapsing render ---
#show "i ther": set text(red)
hi#[ ]#[ ]the#"re"

--- show-text-style-boundary render ---
#show "What's up": set text(blue)
#show "your party": underline
What's #[ ] up at #"your" #text(red)[party?]

--- show-text-within-par render ---
#show "Pythagoras'": highlight
$a^2 + b^2 = c^2$ is Pythagoras' theorem.

--- show-text-outer-space render ---
// Spaces must be interior to strong textual elements for matching to work.
// For outer spaces, it is hard to say whether they would collapse.
#show "a\n": set text(blue)
#show "b\n ": set text(blue)
#show " c ": set text(blue)
a \ #h(0pt, weak: true)
b \ #h(0pt, weak: true)
$x$ c $y$

--- issue-5014-show-text-tags render ---
#{
  let c = counter("c")
  show "b": context c.get().first()
  [a]
  c.step()
  [bc]
}
