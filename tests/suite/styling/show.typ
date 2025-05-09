// Test show rules.

--- show-selector-basic ---
// Override lists.
#show list: it => "(" + it.children.map(v => v.body).join(", ") + ")"

- A
  - B
  - C
- D
- E

--- show-selector-replace-and-show-set ---
// Test full reset.
#show heading: [B]
#show heading: set text(size: 10pt, weight: 400)
A #[= Heading] C

--- show-selector-discard ---
// Test full removal.
#show heading: none

Where is
= There are no headings around here!
my heading?

--- show-selector-realistic ---
// Test integrated example.
#show heading: it => block({
  set text(10pt)
  box(move(dy: -1pt)[📖])
  h(5pt)
  if it.level == 1 {
    underline(text(1.25em, blue, it.body))
  } else {
    text(red, it.body)
  }
})

= Task 1
Some text.

== Subtask
Some more text.

= Task 2
Another text.

--- show-in-show ---
// Test set and show in code blocks.
#show heading: it => {
  set text(red)
  show "ding": [🛎]
  it.body
}

= Heading

--- show-nested-scopes ---
// Test that scoping works as expected.
#{
  let world = [ World ]
  show "W": strong
  world
  {
    set text(blue)
    show: it => {
      show "o": "Ø"
      it
    }
    world
  }
  world
}

--- show-selector-replace ---
#show heading: [1234]
= Heading

--- show-unknown-field ---
// Error: 25-29 heading does not have field "page"
#show heading: it => it.page
= Heading

--- show-text-element-discard ---
#show text: none
Hey

--- show-selector-not-an-element-function ---
// Error: 7-12 only element functions can be used as selectors
#show upper: it => {}

--- show-bad-replacement-type ---
// Error: 16-20 expected content or function, found integer
#show heading: 1234
= Heading

--- show-bad-selector-type ---
// Error: 7-10 expected symbol, string, label, function, regex, or selector, found color
#show red: []

--- show-selector-in-expression ---
// Error: 7-25 show is only allowed directly in code and content blocks
#(1 + show heading: none)

--- show-bare-basic ---
#set page(height: 130pt)
#set text(0.7em)

#align(center)[
  #text(1.3em)[*Essay on typography*] \
  T. Ypst
]

#show: columns.with(2)
#lines(16)

--- show-bare-content-block ---
// Test bare show in content block.
A #[_B #show: c => [*#c*]; C_] D

--- show-bare-vs-set-text ---
// Test style precedence.
#set text(fill: eastern, size: 1.5em)
#show: text.with(fill: forest)
Forest

--- show-bare-replace-with-content ---
#show: [Shown]
Ignored

--- show-bare-in-expression ---
// Error: 4-19 show is only allowed directly in code and content blocks
#((show: body => 2) * body)

--- show-bare-missing-colon-closure ---
// Error: 6 expected colon
#show it => {}

--- show-bare-missing-colon ---
// Error: 6 expected colon
#show it

--- show-recursive-identity ---
// Test basic identity.
#show heading: it => it
= Heading

--- show-multiple-rules ---
// Test more recipes down the chain.
#show list: scale.with(origin: left, x: 80%)
#show heading: []
#show enum: []
- Actual
- Tight
- List
= Nope

--- show-rule-in-function ---
// Test show rule in function.
#let starwars(body) = {
  show list: it => block({
    stack(dir: ltr,
      text(red, it),
      1fr,
      scale(x: -100%, text(blue, it)),
    )
  })
  body
}

- Normal list

#starwars[
  - Star
  - Wars
  - List
]

- Normal list

--- show-recursive-multiple ---
// Test multi-recursion with nested lists.
#set rect(inset: 3pt)
#show list: rect.with(stroke: blue)
#show list: rect.with(stroke: red)
#show list: block

- List
  - Nested
  - List
- Recursive!

--- show-selector-where ---
// Inline code.
#show raw.where(block: false): box.with(
  radius: 2pt,
  outset: (y: 2.5pt),
  inset: (x: 3pt, y: 0pt),
  fill: luma(230),
)

// Code blocks.
#show raw.where(block: true): block.with(
  outset: -3pt,
  inset: 11pt,
  fill: luma(230),
  stroke: (left: 1.5pt + luma(180)),
)

#set page(margin: (top: 12pt))
#set par(justify: true)

This code tests `code`
with selectors and justification.

```rs
code!("it");
```

You can use the ```rs *const T``` pointer or
the ```rs &mut T``` reference.

--- show-set-where-override ---
#show heading: set text(green)
#show heading.where(level: 1): set text(red)
#show heading.where(level: 2): set text(blue)
= Red
== Blue
=== Green

--- show-selector-or-elements-with-set ---
// Looking forward to `heading.where(level: 1 | 2)` :)
#show heading.where(level: 1).or(heading.where(level: 2)): set text(red)
= L1
== L2
=== L3
==== L4

--- show-selector-element-or-label ---
// Test element selector combined with label selector.
#show selector(strong).or(<special>): highlight
I am *strong*, I am _emphasized_, and I am #[special<special>].

--- show-selector-element-or-text ---
// Ensure that text selector cannot be nested in and/or. That's too complicated,
// at least for now.

// Error: 7-41 this selector cannot be used with show
#show heading.where(level: 1).or("more"): set text(red)

--- show-delayed-error ---
// Error: 21-34 panicked with: "hey1"
#show heading: _ => panic("hey1")

// Error: 20-33 panicked with: "hey2"
#show strong: _ => panic("hey2")

= Hello
*strong*

--- issue-5690-oom-par-box ---
// Error: 3:6-5:1 maximum realization iterations exceeded
// Hint: 3:6-5:1 maybe there is a cycle between a show rule that produces content, which is matched by a grouping rule that triggers the show rule
#show par: box

Hello

World
