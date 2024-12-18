// Test counters.

--- counter-basic-1 ---
// Count with string key.
#let mine = counter("mine!")

Final: #context mine.final().at(0) \
#mine.step()
First: #context mine.display() \
#mine.update(7)
#context mine.display("1 of 1", both: true) \
#mine.step()
#mine.step()
Second: #context mine.display("I")
#mine.update(n => n * 2)
#mine.step()

--- counter-basic-2 ---
// Test `counter`.
#let c = counter("heading")
#c.update(2)
#c.update(n => n + 2)
#context test(c.get(), (4,))
#c.update(n => n - 3)
#context test(c.at(here()), (1,))

--- counter-label ---
// Count labels.
#let label = <heya>
#let count = context counter(label).display()
#let elem(it) = [#box(it) #label]

#elem[hey, there!] #count \
#elem[more here!] #count

--- counter-heading ---
// Count headings.
#set heading(numbering: "1.a.")
#show heading: set text(10pt)
#counter(heading).step()

= Alpha
In #context counter(heading).display()
== Beta

#set heading(numbering: none)
= Gamma
#heading(numbering: "I.")[Delta]

At Beta, it was #context {
  let it = query(heading).find(it => it.body == [Beta])
  numbering(it.numbering, ..counter(heading).at(it.location()))
}

--- counter-page ---
#set page(height: 50pt, margin: (bottom: 20pt, rest: 10pt))
#lines(4)
#set page(numbering: "(i)")
#lines(2)
#pagebreak()
#set page(numbering: "1 / 1")
#counter(page).update(1)
#lines(7)

--- counter-page-footer-before-set-page ---
#set page(numbering: "1", margin: (bottom: 20pt))
A
#pagebreak()
#counter(page).update(5)
#set page(fill: aqua)
B

--- counter-page-header-before-set-page ---
#set page(numbering: "1", number-align: top + center, margin: (top: 20pt))
A
#counter(page).update(4)
#set page(fill: aqua)
B

--- counter-page-between-pages ---
// The update happens conceptually between the pages.
#set page(numbering: "1", margin: (bottom: 20pt))
A
#pagebreak()
#counter(page).update(5)
#set page(number-align: top + center, margin: (top: 20pt, bottom: 10pt))
B

--- counter-page-header-only-update ---
// Header should not be affected by default.
// To affect it, put the counter update before the `set page`.
#set page(
  numbering: "1",
  number-align: top + center,
  margin: (top: 20pt),
)

#counter(page).update(5)

--- counter-page-footer-only-update ---
// Footer should be affected by default.
#set page(numbering: "1 / 1", margin: (bottom: 20pt))
#counter(page).update(5)

--- counter-page-display ---
// Counter display should use numbering from style chain.
#set page(
  numbering: "A",
  margin: (bottom: 20pt),
  footer: context align(center, counter(page).display())
)

--- counter-figure ---
// Count figures.
#figure(numbering: "A", caption: [Four 'A's], kind: image, supplement: "Figure")[_AAAA!_]
#figure(numbering: none, caption: [Four 'B's], kind: image, supplement: "Figure")[_BBBB!_]
#figure(caption: [Four 'C's], kind: image, supplement: "Figure")[_CCCC!_]
#counter(figure.where(kind: image)).update(n => n + 3)
#figure(caption: [Four 'D's], kind: image, supplement: "Figure")[_DDDD!_]

--- counter-at-no-context ---
// Test `counter.at` outside of context.
// Error: 2-28 can only be used when context is known
// Hint: 2-28 try wrapping this in a `context` expression
// Hint: 2-28 the `context` expression should wrap everything that depends on this function
#counter("key").at(<label>)

--- issue-2480-counter-reset ---
#let q = counter("question")
#let step-show =  q.step() + context q.display("1")
#let g = grid(step-show, step-show, gutter: 2pt)

#g
#pagebreak()
#step-show
#q.update(10)
#g

--- issue-2480-counter-reset-2 ---
#set block(spacing: 3pt)
#let c = counter("c")
#let foo() = context {
  c.step()
  c.display("1")
  str(c.get().first())
}

#foo()
#block(foo())
#foo()
#foo()
#block(foo())
#block(foo())
#foo()

--- issue-4626-counter-depth-skip ---
// When we step and skip a level, the levels should be filled with zeros, not
// with ones.
#let c = counter("c")
#context test(c.get(), (0,))
#c.step(level: 4)
#context test(c.get(), (0, 0, 0, 1))
#c.step(level: 1)
#context test(c.get(), (1,))
#c.step(level: 3)
#context test(c.get(), (1, 0, 1))
