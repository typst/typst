// Test enumerations.

--- enum-function-call ---
#enum[Embrace][Extend][Extinguish]

--- enum-number-override-nested ---
0. Before first!
1. First.
   2. Indented

+ Second

--- enum-built-in-loop ---
// Test automatic numbering in summed content.
#for i in range(5) {
   [+ #numbering("I", 1 + i)]
}

--- list-mix ---
// Mix of different lists
- Bullet List
+ Numbered List
/ Term: List

--- enum-syntax-at-start ---
// In the line.
1.2 \
This is 0. \
See 0.3. \

--- enum-syntax-edge-cases ---
// Edge cases.
+
Empty \
+Nope \
a + 0.

--- enum-number-override ---
// Test item number overriding.
1. first
+ second
5. fifth

#enum(
   enum.item(1)[First],
   [Second],
   enum.item(5)[Fifth]
)

--- enum-numbering-pattern ---
// Test numbering pattern.
#set enum(numbering: "(1.a.*)")
+ First
+ Second
  2. Nested
     + Deep
+ Normal

--- enum-numbering-full ---
// Test full numbering.
#set enum(numbering: "1.a.", full: true)
+ First
  + Nested

--- enum-numbering-closure ---
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

--- enum-numbering-closure-nested ---
// Test numbering with closure and nested lists.
#set enum(numbering: n => super[#n])
+ A
  + B
+ C

--- enum-numbering-closure-nested-complex ---
// Test numbering with closure and nested lists.
#set text(font: "New Computer Modern")
#set enum(numbering: (..args) => math.mat(args.pos()), full: true)
+ A
  + B
  + C
    + D
+ E
+ F

--- enum-numbering-pattern-empty ---
// Error: 22-24 invalid numbering pattern
#set enum(numbering: "")

--- enum-numbering-pattern-invalid ---
// Error: 22-28 invalid numbering pattern
#set enum(numbering: "(())")

--- issue-2530-enum-item-panic ---
// Enum item (pre-emptive)
#enum.item(none)[Hello]
#enum.item(17)[Hello]
