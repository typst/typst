// Test counters.

--- counter-basic-1 paged ---
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

--- counter-basic-2 paged ---
// Test `counter`.
#let c = counter("heading")
#c.update(2)
#c.update(n => n + 2)
#context test(c.get(), (4,))
#c.update(n => n - 3)
#context test(c.at(here()), (1,))

--- counter-label paged ---
// Count labels.
#let label = <heya>
#let count = context counter(label).display()
#let elem(it) = [#box(it) #label]

#elem[hey, there!] #count \
#elem[more here!] #count

--- counter-heading paged ---
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

--- counter-page paged ---
#set page(height: 50pt, margin: (bottom: 20pt, rest: 10pt))
#lines(4)
#set page(numbering: "(i)")
#lines(2)
#pagebreak()
#set page(numbering: "1 / 1")
#counter(page).update(1)
#lines(7)

--- counter-page-footer-before-set-page paged ---
#set page(numbering: "1", margin: (bottom: 20pt))
A
#pagebreak()
#counter(page).update(5)
#set page(fill: aqua)
B

--- counter-page-header-before-set-page paged ---
#set page(numbering: "1", number-align: top + center, margin: (top: 20pt))
A
#counter(page).update(4)
#set page(fill: aqua)
B

--- counter-page-between-pages paged ---
// The update happens conceptually between the pages.
#set page(numbering: "1", margin: (bottom: 20pt))
A
#pagebreak()
#counter(page).update(5)
#set page(number-align: top + center, margin: (top: 20pt, bottom: 10pt))
B

--- counter-page-header-only-update paged ---
// Header should not be affected by default.
// To affect it, put the counter update before the `set page`.
#set page(
  numbering: "1",
  number-align: top + center,
  margin: (top: 20pt),
)

#counter(page).update(5)

--- counter-page-footer-only-update paged ---
// Footer should be affected by default.
#set page(numbering: "1 / 1", margin: (bottom: 20pt))
#counter(page).update(5)

--- counter-page-display paged ---
// Counter display should use numbering from style chain.
#set page(
  numbering: "A",
  margin: (bottom: 20pt),
  footer: context align(center, counter(page).display())
)

--- counter-matching-numbering paged ---
// Tests that the counter infers the numbering from elements even if there is
// no style chain entry.
#set math.equation(block: true)
#let funcs = (heading, figure, math.equation, footnote)
#show selector.or(..funcs): it => counter(it.func()).display()
#for f in funcs {
  block(f(numbering: "a)")[])
}

--- counter-figure paged ---
// Count figures.
#figure(numbering: "A", caption: [Four 'A's], kind: image, supplement: "Figure")[_AAAA!_]
#figure(numbering: none, caption: [Four 'B's], kind: image, supplement: "Figure")[_BBBB!_]
#figure(caption: [Four 'C's], kind: image, supplement: "Figure")[_CCCC!_]
#counter(figure.where(kind: image)).update(n => n + 3)
#figure(caption: [Four 'D's], kind: image, supplement: "Figure")[_DDDD!_]

--- counter-at-no-context paged ---
// Test `counter.at` outside of context.
// Error: 2-28 can only be used when context is known
// Hint: 2-28 try wrapping this in a `context` expression
// Hint: 2-28 the `context` expression should wrap everything that depends on this function
#counter("key").at(<label>)

--- issue-2480-counter-reset paged ---
#let q = counter("question")
#let step-show =  q.step() + context q.display("1")
#let g = grid(step-show, step-show, gutter: 2pt)

#g
#pagebreak()
#step-show
#q.update(10)
#g

--- issue-2480-counter-reset-2 paged ---
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

--- issue-4626-counter-depth-skip paged ---
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

--- counter-huge paged ---
// Test values greater than 32-bits
#let c = counter("c")
#c.update(100000000001)
#context test(c.get(), (100000000001,))
#c.step()
#context test(c.get(), (100000000002,))
#c.update(n => n + 2)
#context test(c.get(), (100000000004,))

--- counter-rtl paged ---
#set page(width: auto)
#let c = counter("c")
#let s = context c.display() + c.step()
#let tree = [درخت]
#let line = [A #s B #tree #s #tree #s #tree C #s D #s]
#line \
#line
