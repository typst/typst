// Test the `h` and `v` functions.

--- spacing-h-and-v ---
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

--- spacing-rtl ---
// Test RTL spacing.
#set text(dir: rtl)
A #h(10pt) B \
A #h(1fr) B

--- spacing-missing-amount ---
// Missing spacing.
// Error: 10-13 missing argument: amount
Totally #h() ignored

--- issue-3624-spacing-behaviour ---
// Test that metadata after spacing does not force a new paragraph.
#{
  h(1em)
  counter(heading).update(4)
  [Hello ]
  context counter(heading).display()
}

--- trim-weak-space-line-beginning ---
// Weak space at the beginning should be removed.
#h(2cm, weak: true) Hello

--- trim-weak-space-line-end ---
// Weak space at the end of the line should be removed.
#set align(right)
Hello #h(2cm, weak: true)

--- issue-4087 ---
// Weak space at the end of the line is removed.
This is the first line #h(2cm, weak: true) A new line

// Non-weak space consumes a specified width and pushes to next line.
This is the first line #h(2cm, weak: false) A new line

// Similarly, weak space at the beginning of the line is removed.
This is the first line \ #h(2cm, weak: true) A new line

// Non-weak-spacing, on the other hand, is not removed.
This is the first line \ #h(2cm, weak: false) A new line

--- issue-5244-consecutive-weak-space ---
#set par(linebreaks: "optimized")
#{
  [A]
  h(0.3em, weak: true)
  h(0.3em, weak: true)
  [B]
}

--- issue-5244-consecutive-weak-space-heading ---
#set par(justify: true)
#set heading(numbering: "I.")

= #h(0.3em, weak: true) test

--- issue-5253-consecutive-weak-space-math ---
$= thin thin$ a
