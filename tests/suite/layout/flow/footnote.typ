// Test footnotes.

--- footnote-basic ---
#footnote[Hi]

--- footnote-space-collapsing ---
// Test space collapsing before footnote.
A#footnote[A] \
A #footnote[A]

--- footnote-nested ---
First \
Second #footnote[A, #footnote[B, #footnote[C]]]
Third #footnote[D, #footnote[E]] \
Fourth #footnote[F]

--- footnote-nested-break-across-pages ---
#set page(height: 80pt)
A #footnote([I: ] + lines(6) + footnote[II])
B #footnote[III]

--- footnote-entry ---
// Test customization.
#show footnote: set text(red)
#show footnote.entry: set text(8pt, style: "italic")
#set footnote.entry(
  indent: 0pt,
  gap: 0.6em,
  clearance: 0.3em,
  separator: repeat[.],
)

Beautiful footnotes. #footnote[Wonderful, aren't they?]

--- footnote-break-across-pages ---
#set page(height: 200pt)

#lines(2)
#footnote[ // 1
  I
  #footnote[II ...] // 2
]
#lines(6)
#footnote[III: #lines(8, "1")] // 3
#lines(6)
#footnote[IV: #lines(15, "1")] // 4
#lines(6)
#footnote[V] // 5

--- footnote-break-across-pages-block ---
#set page(height: 100pt)
#block[
  #lines(3) #footnote(lines(6, "1"))
  #footnote[Y]
  #footnote[Z]
]

--- footnote-break-across-pages-float ---
#set page(height: 180pt)

#lines(5)

#place(
  bottom,
  float: true,
  rect(height: 50pt, width: 100%, {
    footnote(lines(6, "1"))
    footnote(lines(2, "I"))
  })
)

#lines(5)

--- footnote-break-across-pages-nested ---
#set page(height: 120pt)
#block[
  #lines(4)
  #footnote[
    #lines(6, "1")
    #footnote(lines(3, "I"))
  ]
]

--- footnote-in-columns ---
#set page(height: 120pt, columns: 2)

#place(
  top + center,
  float: true,
  scope: "parent",
  clearance: 12pt,
  strong[Title],
)

#lines(3)
#footnote(lines(4, "1"))

#lines(2)
#footnote(lines(2, "1"))

--- footnote-in-list ---
#set page(height: 120pt)

- A #footnote[a]
- B #footnote[b]
- C #footnote[c]
- D #footnote[d]
- E #footnote[e]
- F #footnote[f]
- G #footnote[g]

--- footnote-block-at-end ---
#set page(height: 50pt)
A
#block(footnote[hello])

--- footnote-block-fr ---
#set page(height: 110pt)
A
#block(width: 100%, height: 1fr, fill: aqua)[
  B #footnote[I] #footnote[II]
]
C

--- footnote-float-priority ---
#set page(height: 100pt)

#lines(3)

#place(
  top,
  float: true,
  rect(height: 40pt)
)

#block[
  V
  #footnote[1]
  #footnote[2]
  #footnote[3]
  #footnote[4]
]

#lines(5)

--- footnote-in-caption ---
// Test footnote in caption.
Read the docs #footnote[https://typst.app/docs]!
#figure(
  image("/assets/images/graph.png", width: 70%),
  caption: [
    A graph #footnote[A _graph_ is a structure with nodes and edges.]
  ]
)
More #footnote[just for ...] footnotes #footnote[... testing. :)]

--- footnote-in-place ---
A
#place(top + right, footnote[A])
#figure(
  placement: bottom,
  caption: footnote[B],
  rect(),
)

--- footnote-duplicate ---
// Test duplicate footnotes.
#let lang = footnote[Languages.]
#let nums = footnote[Numbers.]

/ "Hello": A word #lang
/ "123": A number #nums

- "Hello" #lang
- "123" #nums

+ "Hello" #lang
+ "123" #nums

#table(
  columns: 2,
  [Hello], [A word #lang],
  [123], [A number #nums],
)

--- footnote-invariant ---
// Ensure that a footnote and the first line of its entry
// always end up on the same page.
#set page(height: 120pt)

#lines(5)

A #footnote(lines(6, "1"))

--- footnote-ref ---
// Test references to footnotes.
A footnote #footnote[Hi]<fn> \
A reference to it @fn

--- footnote-self-ref ---
// Error: 2-16 footnote cannot reference itself
#footnote(<fn>) <fn>

--- footnote-ref-multiple ---
// Multiple footnotes are refs
First #footnote[A]<fn1> \
Second #footnote[B]<fn2> \
First ref @fn1 \
Third #footnote[C] \
Fourth #footnote[D]<fn4> \
Fourth ref @fn4 \
Second ref @fn2 \
Second ref again @fn2

--- footnote-ref-forward ---
// Forward reference
Usage @fn \
Definition #footnote[Hi]<fn>

--- footnote-ref-in-footnote ---
// Footnote ref in footnote
#footnote[Reference to next @fn]
#footnote[Reference to myself @fn]<fn>
#footnote[Reference to previous @fn]

--- footnote-styling ---
// Styling
#show footnote: text.with(fill: red)
Real #footnote[...]<fn> \
Ref @fn

--- footnote-ref-call ---
// Footnote call with label
#footnote(<fn>)
#footnote[Hi]<fn>
#ref(<fn>)
#footnote(<fn>)

--- footnote-in-table ---
// Test footnotes in tables. When the table spans multiple pages, the footnotes
// will all be after the table, but it shouldn't create any empty pages.
#set page(height: 100pt)

= Tables
#table(
  columns: 2,
  [Hello footnote #footnote[This is a footnote.]],
  [This is more text],
  [This cell
   #footnote[This footnote is not on the same page]
   breaks over multiple pages.],
  image("/assets/images/tiger.jpg"),
)

#table(
  columns: 3,
  ..range(1, 10)
    .map(numbering.with("a"))
    .map(v => upper(v) + footnote(v))
)

--- footnote-multiple-in-one-line ---
#set page(height: 100pt)
#v(50pt)
A #footnote[a]
B #footnote[b]

--- issue-1433-footnote-in-list ---
// Test that footnotes in lists do not produce extraneous page breaks. The list
// layout itself does not currently react to the footnotes layout, weakening the
// "footnote and its entry are on the same page" invariant somewhat, but at
// least there shouldn't be extra page breaks.
#set page(height: 100pt)
#block(height: 50pt, width: 100%, fill: aqua)

- #footnote[1]
- #footnote[2]

--- issue-footnotes-skip-first-page ---
// In this issue, we would get an empty page at the beginning because footnote
// layout didn't properly check for in_last.
#set page(height: 50pt)
#footnote[A]
#footnote[B]

--- issue-4454-footnote-ref-numbering ---
// Test that footnote references are numbered correctly.
A #footnote(numbering: "*")[B]<fn>, C @fn, D @fn, E @fn.

--- issue-5354-footnote-empty-frame-infinite-loop ---
// Test whether an empty footnote would cause infinite loop
#show footnote.entry: it => {}
#lorem(3) #footnote[A footnote]

--- issue-5256-multiple-footnotes-in-footnote ---
// Test whether all footnotes inside another footnote are listed.
#footnote[#footnote[A]#footnote[B]#footnote[C]]

--- issue-5435-footnote-migration-in-floats ---
// Test that a footnote should not prompt migration when in a float that was
// queued to the next page (due to the float being too large), even if the
// footnote does not fit, breaking the footnote invariant.
#set page(height: 50pt)

#place(
  top,
  float: true,
  {
    v(100pt)
    footnote[a]
  }
)
#place(
  top,
  float: true,
  footnote[b]
)
