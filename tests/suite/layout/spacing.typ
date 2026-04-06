// Test the `h` and `v` functions.

--- spacing-h-and-v paged ---
// Linebreak and leading-sized weak spacing are equivalent.
#box[A \ B] #box[A #v(0.65em, weak: true) B]

// Eating up soft spacing.
Inv#h(0pt)isible

// Multiple spacings in a row.
Add #h(10pt) #h(10pt) up

// Relative to area.
#let x = 25% - 4pt
|#h(x)|#h(x)|#h(x)|#h(x)|

// Fractional.
| #h(1fr) | #h(2fr) | #h(1fr) |

--- spacing-rtl paged ---
// Test RTL spacing.
#set text(dir: rtl)
A #h(10pt) B \
A #h(1fr) B

--- spacing-weak-versus-block-spacing paged ---
// Weak spacing wins against block spacing.
0
#v(1cm, weak: true)
#block(above: 2cm, below: 0pt, height: 0pt)
1
#v(1cm, weak: true)
2

--- spacing-fr-weak-collapse paged ---
// Fractional weak spacing should collapse like rel weak spacing.
#set page(height: 100pt)
0
#v(1fr, weak: true)
#v(1fr, weak: false)
1
#v(1fr, weak: false)

--- spacing-fr-weak-standalone-start paged ---
// Standalone fractional weak spacing should collapse.
#set page(height: 60pt)
#v(1fr, weak: true)
0

--- spacing-fr-weak-standalone-end paged ---
// Standalone factional weak spacing at the end should collapse..
#set page(height: 60pt)
#v(1fr, weak: false)
1
#v(1fr, weak: true)

--- spacing-fr-weak-destructs-rel-abs paged ---
// Fr spacing destructs weak rel/abs spacing.
#set page(height: 100pt)
0
#v(1fr, weak: true)
#v(1em, weak: true)
1
#v(1fr, weak: true)
#v(2em, weak: true)
2

--- spacing-fr-weak-destructs-smaller paged ---
// Larger fr destructs smaller fr.
#set page(height: 100pt)
0
#v(1fr, weak: true)
#v(2fr, weak: true) // wins
2
#v(2fr, weak: true)
#v(4fr, weak: true) // wins
6

--- spacing-fr-weak-survives-with-strong paged ---
// Weak fr survives with strong fr, like weak rel survives with strong rel.
#set page(height: 100pt)
#v(1fr, weak: false)
#v(1fr, weak: true)
2
#v(1fr, weak: false)
#v(2fr, weak: true)
4

--- spacing-fr-weak-with-fr-block paged ---
#set page(height: 150pt)
0
#v(2fr, weak: true)
2
#v(1fr, weak: true)
#block(spacing: 0pt, height: 1fr, fill: aqua)[A]
#v(4fr, weak: true)
8

--- spacing-fr-weak-versus-fr-block-spacing paged ---
// Weak fr spacing wins against fr block spacing, just like for weak rel
// spacing.
#set page(height: 100pt)
0
#v(1fr, weak: true)
#block(above: 2fr, below: 0pt, height: 0pt)
1
#v(1fr, weak: true)
2

--- spacing-missing-amount eval ---
// Missing spacing.
// Error: 10-13 missing argument: amount
Totally #h() ignored

--- issue-3624-spacing-behaviour paged ---
// Test that metadata after spacing does not force a new paragraph.
#{
  h(1em)
  counter(heading).update(4)
  [Hello ]
  context counter(heading).display()
}

--- trim-weak-space-line-beginning paged ---
// Weak space at the beginning should be removed.
#h(2cm, weak: true) Hello

--- trim-weak-space-line-end paged ---
// Weak space at the end of the line should be removed.
#set align(right)
Hello #h(2cm, weak: true)

--- issue-4087 paged ---
// Weak space at the end of the line is removed.
This is the first line #h(2cm, weak: true) A new line

// Non-weak space consumes a specified width and pushes to next line.
This is the first line #h(2cm, weak: false) A new line

// Similarly, weak space at the beginning of the line is removed.
This is the first line \ #h(2cm, weak: true) A new line

// Non-weak-spacing, on the other hand, is not removed.
This is the first line \ #h(2cm, weak: false) A new line

--- issue-5244-consecutive-weak-space paged ---
#set par(linebreaks: "optimized")
#{
  [A]
  h(0.3em, weak: true)
  h(0.3em, weak: true)
  [B]
}

--- issue-5244-consecutive-weak-space-heading paged ---
#set par(justify: true)
#set heading(numbering: "I.")

= #h(0.3em, weak: true) test

--- issue-5253-consecutive-weak-space-math paged ---
$= thin thin$ a
