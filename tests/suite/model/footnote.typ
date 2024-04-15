// Test footnotes.

--- footnote-basic ---
#footnote[Hi]

--- footnote-space-collapsing ---
// Test space collapsing before footnote.
A#footnote[A] \
A #footnote[A]

--- footnote-nested ---
// Test nested footnotes.
First \
Second #footnote[A, #footnote[B, #footnote[C]]] \
Third #footnote[D, #footnote[E]] \
Fourth

--- footnote-nested-same-frame ---
// Currently, numbers a bit out of order if a nested footnote ends up in the
// same frame as another one. :(
#footnote[A, #footnote[B]], #footnote[C]

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
// LARGE
#set page(height: 200pt)

#lorem(5)
#footnote[ // 1
  A simple footnote.
  #footnote[Well, not that simple ...] // 2
]
#lorem(15)
#footnote[Another footnote: #lorem(30)] // 3
#lorem(15)
#footnote[My fourth footnote: #lorem(50)] // 4
#lorem(15)
#footnote[And a final footnote.] // 5

--- footnote-in-columns ---
// Test footnotes in columns, even those that are not enabled via `set page`.
#set page(height: 120pt)
#align(center, strong[Title])
#show: columns.with(2)
#lorem(3) #footnote(lorem(6))
Hello there #footnote(lorem(2))

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

#lorem(13)

There #footnote(lorem(20))

--- footnote-ref ---
// Test references to footnotes.
A footnote #footnote[Hi]<fn> \
A reference to it @fn

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

--- issue-multiple-footnote-in-one-line ---
// Test that the logic that keeps footnote entry together with
// their markers also works for multiple footnotes in a single
// line or frame (here, there are two lines, but they are one
// unit due to orphan prevention).
#set page(height: 100pt)
#v(40pt)
A #footnote[a] \
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
