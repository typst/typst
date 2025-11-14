// Test bullet lists.

--- list-basic paged ---
_Shopping list_
#list[Apples][Potatoes][Juice]

--- list-nested paged ---
- First level.

  - Second level.
    There are multiple paragraphs.

    - Third level.

    Still the same bullet point.

  - Still level 2.

- At the top.

--- list-content-block paged ---
- Level 1
  - Level #[
2 through content block
]

--- list-top-level-indent paged ---
  - Top-level indent
- is fine.

--- list-indent-specifics paged pdftags pdfstandard(ua-1) ---
 - A
     - B
   - C
- D

--- list-indent-trivia-nesting paged ---
// Test indent nesting behavior with odd trivia (comments and spaces). The
// comments should _not_ affect the nesting. Only the final column matters.

#let indented = [
- a
 /**/- b
/**/ - c
   /*spanning
     multiple
      lines */ - d
    - e
/**/       - f
/**/  - g
]

#let item = list.item
#let manual = {
  [ ]
  item({
    [a]
    [ ]
    item[b]
    [ ]; [ ]
    item({
      [c]
      [ ]; [ ]
      item[d]
    })
    [ ]
    item({
      [e]
      [ ]; [ ]
      item[f]
      [ ]; [ ]
      item[g]
    })
  })
  [ ]
}

#test(indented, manual)

--- list-indent-bracket-nesting paged ---
// Test list indent nesting behavior when directly at a starting bracket.

#let indented = {
  [- indented
  - less
  ]
  [- indented
   - same
  - then less
   - then same
  ]
  [- indented
    - more
   - then same
  - then less
  ]
}

#let item = list.item
#let manual = {
    {
      item[indented]; [ ]
      item[less]; [ ]
    }
    {
      item[indented]; [ ]
      item[same]; [ ]
      item[then less #{
        item[then same]
      }]; [ ]
    }
    {
      item[indented #{
        item[more]
      }]; [ ]
      item[then same]; [ ]
      item[then less]; [ ]
    }
}

#test(indented, manual)

--- list-tabs paged ---
// This works because tabs are used consistently.
	- A with 1 tab
		- B with 2 tabs

--- list-mixed-tabs-and-spaces paged ---
// This doesn't work because of mixed tabs and spaces.
  - A with 2 spaces
		- B with 2 tabs

--- list-syntax-edge-cases paged ---
// Edge cases.
-
Not in list
-Nope

--- list-marker-align-unaffected paged ---
// Alignment shouldn't affect marker
#set align(horizon)

- ABCDEF\ GHIJKL\ MNOPQR

--- list-marker-dash paged ---
// Test en-dash.
#set list(marker: [--])
- A
- B

--- list-marker-cycle paged ---
// Test that items are cycled.
#set list(marker: ([--], [•]))
- A
  - B
    - C

--- list-marker-closure paged ---
// Test function.
#set list(marker: n => if n == 1 [--] else [•])
- A
- B
  - C
  - D
    - E
- F

--- list-marker-bare-hyphen paged ---
// Test that bare hyphen doesn't lead to cycles and crashes.
#set list(marker: [-])
- Bare hyphen is
- a bad marker

--- list-marker-array-empty paged ---
// Error: 19-21 array must contain at least one marker
#set list(marker: ())

--- list-attached paged ---
// Test basic attached list.
Attached to:
- the bottom
- of the paragraph

Next paragraph.

--- list-attached-above-spacing paged ---
// Test that attached list isn't affected by block spacing.
#show list: set block(above: 100pt)
Hello
- A
World
- B

--- list-non-attached-followed-by-attached paged ---
// Test non-attached list followed by attached list,
// separated by only word.
Hello

- A

World
- B

--- list-tight-non-attached-tight paged ---
// Test non-attached tight list.
#set block(spacing: 15pt)
Hello
- A
World

- B
- C

More.

--- list-wide-cannot-attach paged ---
// Test that wide lists cannot be ...
#set par(spacing: 15pt)
Hello
- A

- B
World

--- list-wide-really-cannot-attach paged ---
// ... even if forced to.
Hello
#list(tight: false)[A][B]
World

--- list-items-context paged ---
#context [+ A]
#context [+ B]
#context [+ C]

--- list-item-styling paged ---
- Hello
#text(red)[- World]
#text(green)[- What up?]

--- list-par paged html ---
// Check whether the contents of list items become paragraphs.
#show par: it => if target() != "html" { highlight(it) } else { it }

#block[
  // No paragraphs.
  - Hello
  - World
]

#block[
  - Hello // Paragraphs

    From
  - World // No paragraph because it's a tight list.
]

#block[
  - Hello // Paragraphs either way

    From

    The

  - World // Paragraph because it's a wide list.
]

--- issue-2530-list-item-panic paged ---
// List item (pre-emptive)
#list.item[Hello]

--- issue-1850-list-attach-spacing paged ---
// List attachment should only work with paragraphs, not other blocks.
#set page(width: auto)
#let part = box.with(stroke: 1pt, inset: 3pt)
#{
  part[
    $ x $
    - A
  ]
  part($ x $ + list[A])
  part($ x $ + list[ A ])
  part[
    $ x $

    - A
  ]
  part($ x $ + parbreak() + list[A])
  part($ x $ + parbreak() + parbreak() + list[A])
}

--- issue-5503-list-in-align paged ---
// `align` is block-level and should interrupt a list.
#show list: [List]
- a
- b
#align(right)[- i]
- j

--- issue-5719-list-nested paged ---
// Lists can be immediately nested.
- A
- - B
  - C
- = D
  E

--- issue-6242-tight-list-attach-spacing paged ---
// Nested tight lists should be uniformly spaced when list spacing is set.
#set list(spacing: 1.2em)
- A
  - B
  - C
- C
