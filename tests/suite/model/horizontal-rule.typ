// Test horizontal rules.

--- horizontal-rule-basic paged ---
// Basic horizontal rule.
#set page(width: 200pt)
Before
#horizontal-rule()
After

--- horizontal-rule-stroke paged ---
// Test stroke customization.
#set page(width: 200pt)
#horizontal-rule(stroke: 2pt + red)
#v(5pt)
#horizontal-rule(stroke: 1pt + blue)
#v(5pt)
#horizontal-rule(stroke: (paint: green, thickness: 3pt, dash: "dashed"))

--- horizontal-rule-set-rule paged ---
// Test set rules.
#set page(width: 200pt)
#set horizontal-rule(stroke: 2pt + maroon)
Before
#horizontal-rule()
After

--- horizontal-rule-show-rule paged ---
// Test show rules.
#set page(width: 200pt)
#show horizontal-rule: set block(above: 20pt, below: 20pt)
Before
#horizontal-rule()
After

--- horizontal-rule-multiple paged ---
// Test multiple horizontal rules.
#set page(width: 200pt)
Section 1
#horizontal-rule()
Section 2
#horizontal-rule()
Section 3

--- horizontal-rule-in-container paged ---
// Test horizontal rule in a container.
#set page(width: 200pt)
#box(width: 100pt, stroke: 1pt)[
  Content
  #horizontal-rule()
  More content
]

--- horizontal-rule-styled-show paged ---
// Test styling via show rule.
#set page(width: 200pt)
#show horizontal-rule: it => block(above: 1em, below: 1em)[
  #set align(center)
  #box(width: 80%, it)
]
Before
#horizontal-rule()
After
