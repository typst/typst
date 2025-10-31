// Test enumerations.

--- enum-function-call render ---
#enum[Embrace][Extend][Extinguish]

--- enum-number-override-nested render pdftags ---
0. Before first!
1. First.
   2. Indented

+ Second

--- enum-built-in-loop render ---
// Test automatic numbering in summed content.
#for i in range(5) {
   [+ #numbering("I", 1 + i)]
}

--- list-mix render ---
// Mix of different lists
- Bullet List
+ Numbered List
/ Term: List

--- enum-syntax-at-start render ---
// In the line.
1.2 \
This is 0. \
See 0.3. \

--- enum-syntax-edge-cases render ---
// Edge cases.
+
Empty \
+Nope \
a + 0.

--- enum-syntax-number-length render ---
// Ensure that indentation works from the beginning of a number, not the end.

10. a
   11. b
 12. c // same level as b
  13. d // indented past c
14. e

--- enum-number-override render ---
// Test item number overriding.
1. first
+ second
5. fifth

#enum(
   enum.item(1)[First],
   [Second],
   enum.item(5)[Fifth]
)

--- enum-item-number-optional render ---
#enum.item[First]
#enum.item[Second]

--- enum-numbering-pattern render ---
// Test numbering pattern.
#set enum(numbering: "(1.a.*)")
+ First
+ Second
  2. Nested
     + Deep
+ Normal

--- enum-numbering-full render ---
// Test full numbering.
#set enum(numbering: "1.a.", full: true)
+ First
  + Nested

--- enum-numbering-reversed render ---
// Test reverse numbering.
#set enum(reversed: true)
+ Coffee
+ Tea
+ Milk

--- enum-numbering-reversed-overridden render ---
// Test reverse numbering with overridden numbers.
#set enum(reversed: true)
+ A
+ B
+ C
9. D
+ E
+ F

--- enum-numbering-closure render ---
// Test numbering with closure.
#enum(
  start: 3,
  spacing: 0.65em - 3pt,
  tight: false,
  numbering: n => text(
    fill: (red, green, blue).at(calc.rem(n, 3)),
    numbering("A", n),
  ),
  [Red], [Green], [Blue], [Red],
)

--- enum-start html ---
#enum(
  start: 3,
  [Skipping],
  [Ahead],
)

--- enum-numbering-closure-nested render ---
// Test numbering with closure and nested lists.
#set enum(numbering: n => super[#n])
+ A
  + B
+ C

--- enum-numbering-closure-nested-complex render ---
// Test numbering with closure and nested lists.
#set text(font: "New Computer Modern")
#set enum(numbering: (..args) => math.mat(args.pos()), full: true)
+ A
  + B
  + C
    + D
+ E
+ F

--- enum-numbering-pattern-empty render ---
// Error: 22-24 invalid numbering pattern
#set enum(numbering: "")

--- enum-numbering-pattern-invalid render ---
// Error: 22-28 invalid numbering pattern
#set enum(numbering: "(())")

--- enum-numbering-huge render ---
// Test values greater than 32-bits
100000000001. A
+             B

--- enum-number-align-unaffected render ---
// Alignment shouldn't affect number
#set align(horizon)

+ ABCDEF\ GHIJKL\ MNOPQR
   + INNER\ INNER\ INNER
+ BACK\ HERE

--- enum-number-align-default render ---
// Enum number alignment should be 'end' by default
1. a
10. b
100. c

--- enum-number-align-specified render ---
#set enum(number-align: start)
1.  a
8.  b
16. c

--- enum-number-align-2d render ---
#set enum(number-align: center + horizon)
1.  #box(fill: teal, inset: 10pt )[a]
8.  #box(fill: teal, inset: 10pt )[b]
16. #box(fill: teal,inset: 10pt )[c]

--- enum-number-align-unfolded render ---
// Number align option should not be affected by the context.
#set align(center)
#set enum(number-align: start)

4.  c
8.  d
16. e\ f
   2.  f\ g
   32. g
   64. h

--- enum-number-align-values render ---
// Test valid number align values (horizontal and vertical)
#set enum(number-align: start)
#set enum(number-align: end)
#set enum(number-align: left)
#set enum(number-align: center)
#set enum(number-align: right)
#set enum(number-align: top)
#set enum(number-align: horizon)
#set enum(number-align: bottom)

--- enum-par render html ---
// Check whether the contents of enum items become paragraphs.
#show par: it => if target() != "html" { highlight(it) } else { it }

// No paragraphs.
#block[
  + Hello
  + World
]

#block[
  + Hello // Paragraphs

    From
  + World // No paragraph because it's a tight enum
]

#block[
  + Hello // Paragraphs

    From

    The

  + World // Paragraph because it's a wide enum
]

--- issue-2530-enum-item-panic render ---
// Enum item (pre-emptive)
#enum.item(auto)[Hello]
#enum.item(17)[Hello]

--- issue-5503-enum-in-align render ---
// `align` is block-level and should interrupt an enum.
+ a
+ b
#align(right)[+ c]
+ d

--- issue-5719-enum-nested render ---
// Enums can be immediately nested.
1. A
2. 1. B
   2. C
+ + D
  + E
+ = F
  G
