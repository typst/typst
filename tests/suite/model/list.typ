// Test bullet lists.

--- list-basic render ---
_Shopping list_
#list[Apples][Potatoes][Juice]

--- list-nested render ---
- First level.

  - Second level.
    There are multiple paragraphs.

    - Third level.

    Still the same bullet point.

  - Still level 2.

- At the top.

--- list-content-block render ---
- Level 1
  - Level #[
2 through content block
]

--- list-top-level-indent render ---
  - Top-level indent
- is fine.

--- list-indent-specifics render pdftags ---
 - A
     - B
   - C
- D

--- list-indent-trivia-nesting render ---
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

--- list-indent-bracket-nesting render ---
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

--- list-tabs render ---
// This works because tabs are used consistently.
	- A with 1 tab
		- B with 2 tabs

--- list-mixed-tabs-and-spaces render ---
// This doesn't work because of mixed tabs and spaces.
  - A with 2 spaces
		- B with 2 tabs

--- list-syntax-edge-cases render ---
// Edge cases.
-
Not in list
-Nope

--- list-marker-align-unaffected render ---
// Alignment shouldn't affect marker
#set align(horizon)

- ABCDEF\ GHIJKL\ MNOPQR

--- list-marker-dash render ---
// Test en-dash.
#set list(marker: [--])
- A
- B

--- list-marker-cycle render ---
// Test that items are cycled.
#set list(marker: ([--], [•]))
- A
  - B
    - C

--- list-marker-closure render ---
// Test function.
#set list(marker: n => if n == 1 [--] else [•])
- A
- B
  - C
  - D
    - E
- F

--- list-marker-bare-hyphen render ---
// Test that bare hyphen doesn't lead to cycles and crashes.
#set list(marker: [-])
- Bare hyphen is
- a bad marker

--- list-marker-array-empty render ---
// Error: 19-21 array must contain at least one marker
#set list(marker: ())

--- list-attached render ---
// Test basic attached list.
Attached to:
- the bottom
- of the paragraph

Next paragraph.

--- list-attached-above-spacing render ---
// Test that attached list isn't affected by block spacing.
#show list: set block(above: 100pt)
Hello
- A
World
- B

--- list-non-attached-followed-by-attached render ---
// Test non-attached list followed by attached list,
// separated by only word.
Hello

- A

World
- B

--- list-tight-non-attached-tight render ---
// Test non-attached tight list.
#set block(spacing: 15pt)
Hello
- A
World

- B
- C

More.

--- list-wide-cannot-attach render ---
// Test that wide lists cannot be ...
#set par(spacing: 15pt)
Hello
- A

- B
World

--- list-wide-really-cannot-attach render ---
// ... even if forced to.
Hello
#list(tight: false)[A][B]
World

--- list-items-context render ---
#context [+ A]
#context [+ B]
#context [+ C]

--- list-item-styling render ---
- Hello
#text(red)[- World]
#text(green)[- What up?]

--- list-par render html ---
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

--- issue-2530-list-item-panic render ---
// List item (pre-emptive)
#list.item[Hello]

--- issue-1850-list-attach-spacing render ---
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

--- issue-5503-list-in-align render ---
// `align` is block-level and should interrupt a list.
#show list: [List]
- a
- b
#align(right)[- i]
- j

--- issue-5719-list-nested render ---
// Lists can be immediately nested.
- A
- - B
  - C
- = D
  E

--- issue-6242-tight-list-attach-spacing render ---
// Nested tight lists should be uniformly spaced when list spacing is set.
#set list(spacing: 1.2em)
- A
  - B
  - C
- C
