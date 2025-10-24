// General tests for set.

--- set-instantiation-site render ---
// Test that text is affected by instantiation-site bold.
#let x = [World]
Hello *#x*

--- set-instantiation-site-markup render ---
// Test that lists are affected by correct indents.
#let fruit = [
  - Apple
  - Orange
  #list(body-indent: 20pt)[Pear]
]

- Fruit
#[#set list(indent: 10pt)
 #fruit]
- No more fruit

--- set-text-override render ---
// Test that block spacing and text style are respected from
// the outside, but the more specific fill is respected.
#set par(spacing: 4pt)
#set text(style: "italic", fill: eastern)
#let x = [And the forest #parbreak() lay silent!]
#text(fill: forest, x)

--- set-scoped-in-code-block render ---
// Test that scoping works as expected.
#{
  if true {
    set text(blue)
    [Blue ]
  }
  [Not blue]
}

--- closure-path-resolve-in-layout-phase render ---
// Test relative path resolving in layout phase.
#let choice = ("monkey.svg", "rhino.png", "tiger.jpg")
#set enum(numbering: n => {
  let path = "/assets/images/" + choice.at(n - 1)
  move(dy: -0.15em, image(path, width: 1em, height: 1em))
})

+ Monkey
+ Rhino
+ Tiger

--- set-if render ---
// Test conditional set.
#show ref: it => {
  set text(red) if it.target == <unknown>
  "@" + str(it.target)
}

@hello from the @unknown

--- set-if-bad-type render ---
// Error: 19-24 expected boolean, found integer
#set text(red) if 1 + 2

--- set-in-expr render ---
// Error: 12-26 set is only allowed directly in code and content blocks
#{ let x = set text(blue) }

--- set-bad-trivia render ---
// Error cases parsing set rules with trivia between the function and args.
// Error: 10 expected argument list
#set page
         (numbering: "1")
// Error: 10 expected argument list
// Hint: 10 there may not be any spaces before the argument list
#set page (numbering: "2")
// Error: 10 expected argument list
// Hint: 10 there may not be any spaces before the argument list
#set page/**/(numbering: "3")

// This is fine though
#set/**/page(numbering: "4")

--- set-vs-construct-1 render ---
// Ensure that constructor styles aren't passed down the tree.
// The inner list should have no extra indent.
#set par(leading: 2pt)
#list(body-indent: 20pt, [First], list[A][B])

--- set-vs-construct-2 render ---
// Ensure that constructor styles win, but not over outer styles.
// The outer paragraph should be right-aligned,
// but the B should be center-aligned.
#set list(marker: [>])
#list(marker: [--])[
  #rect(width: 2cm, fill: conifer, inset: 4pt, list[A])
]

--- set-vs-construct-3 render ---
// The inner rectangle should also be yellow here.
// (and therefore invisible)
#[#set rect(fill: yellow);#text(1em, rect(inset: 5pt, rect()))]

--- set-vs-construct-4 render ---
// The inner rectangle should not be yellow here.
A #box(rect(fill: yellow, inset: 5pt, rect())) B

--- show-set-vs-construct render ---
// The constructor property should still work
// when there are recursive show rules.
#show enum: set text(blue)
#enum(numbering: "(a)", [A], enum[B])
