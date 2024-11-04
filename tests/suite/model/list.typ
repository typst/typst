// Test bullet lists.

--- list-basic ---
_Shopping list_
#list[Apples][Potatoes][Juice]

--- list-nested ---
- First level.

  - Second level.
    There are multiple paragraphs.

    - Third level.

    Still the same bullet point.

  - Still level 2.

- At the top.

--- list-content-block ---
- Level 1
  - Level #[
2 through content block
]

--- list-top-level-indent ---
  - Top-level indent
- is fine.

--- list-indent-specifics ---
 - A
     - B
   - C
- D

--- list-indent-trivia-nesting ---
// Test indent nesting behavior with odd trivia (comments and spaces).

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
// Current behavior is that list columns are based on the first non-whitespace
// element in their line, so the block comments here determine the column the
// list starts at

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

--- list-tabs ---
// This works because tabs are used consistently.
	- A with 1 tab
		- B with 2 tabs

--- list-mixed-tabs-and-spaces ---
// This doesn't work because of mixed tabs and spaces.
  - A with 2 spaces
		- B with 2 tabs

--- list-syntax-edge-cases ---
// Edge cases.
-
Not in list
-Nope

--- list-marker-align-unaffected ---
// Alignment shouldn't affect marker
#set align(horizon)

- ABCDEF\ GHIJKL\ MNOPQR

--- list-marker-dash ---
// Test en-dash.
#set list(marker: [--])
- A
- B

--- list-marker-cycle ---
// Test that items are cycled.
#set list(marker: ([--], [•]))
- A
  - B
    - C

--- list-marker-closure ---
// Test function.
#set list(marker: n => if n == 1 [--] else [•])
- A
- B
  - C
  - D
    - E
- F

--- list-marker-bare-hyphen ---
// Test that bare hyphen doesn't lead to cycles and crashes.
#set list(marker: [-])
- Bare hyphen is
- a bad marker

--- list-marker-array-empty ---
// Error: 19-21 array must contain at least one marker
#set list(marker: ())

--- list-attached ---
// Test basic attached list.
Attached to:
- the bottom
- of the paragraph

Next paragraph.

--- list-attached-above-spacing ---
// Test that attached list isn't affected by block spacing.
#show list: set block(above: 100pt)
Hello
- A
World
- B

--- list-non-attached-followed-by-attached ---
// Test non-attached list followed by attached list,
// separated by only word.
Hello

- A

World
- B

--- list-tight-non-attached-tight ---
// Test non-attached tight list.
#set block(spacing: 15pt)
Hello
- A
World

- B
- C

More.

--- list-wide-cannot-attach ---
// Test that wide lists cannot be ...
#set par(spacing: 15pt)
Hello
- A

- B
World

--- list-wide-really-cannot-attach ---
// ... even if forced to.
Hello
#list(tight: false)[A][B]
World

--- list-items-context ---
#context [+ A]
#context [+ B]
#context [+ C]

--- list-item-styling ---
- Hello
#text(red)[- World]
#text(green)[- What up?]

--- issue-2530-list-item-panic ---
// List item (pre-emptive)
#list.item[Hello]

--- issue-1850-list-attach-spacing ---
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
