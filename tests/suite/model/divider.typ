// Test divider element.

--- divider-basic paged ---
// Basic divider.
#set page(width: 200pt)
Before
#divider()
After

--- divider-multiple paged ---
// Test multiple dividers.
#set page(width: 200pt)
Section 1
#divider()
Section 2
#divider()
Section 3

--- divider-in-container paged ---
// Test divider in a container.
#set page(width: 200pt)
#box(width: 150pt, stroke: 1pt, inset: 10pt)[
  Content before
  #divider()
  Content after
]

--- divider-show-custom-line paged ---
// Test custom line styling
#set page(width: 200pt)
#show divider: block[#line(length: 100%, stroke: 2pt + red)]
Before
#divider()
After

--- divider-show-centered paged ---
// Test centered, shorter divider via show rule.
#set page(width: 200pt)
#show divider: block(width: 100%, spacing: 1em)[
  #align(center)[
    #line(length: 80%, stroke: 0.05em)
  ]
]
Before
#divider()
After

--- divider-show-decorative paged ---
// Test non-line content (asterism style).
#set page(width: 200pt)
#show divider: block(spacing: 2em)[
  #align(center)[∗ ∗ ∗]
]
Chapter 1
#divider()
Chapter 2

--- divider-html html ---
// Test HTML output (should map to <hr>).
Introduction
#divider()
Body text
