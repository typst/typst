// Test context expressions.

--- context-body-atomic-in-markup ---
// Test that context body is parsed as atomic expression.
#let c = [#context "hello".]
#test(c.children.first().func(), (context none).func())
#test(c.children.last(), [.])

--- context-element-constructor-forbidden ---
// Test that manual construction is forbidden.
// Error: 2-25 cannot be constructed manually
#(context none).func()()

--- context-in-show-rule ---
// Test that show rule establishes context.
#set heading(numbering: "1.")
#show heading: it => test(
  counter(heading).get(),
  (intro: (1,), back: (2,)).at(str(it.label)),
)

= Introduction <intro>
= Background <back>

--- context-in-show-rule-query ---
// Test that show rule on non-locatable element allows `query`.
// Error: 18-47 Assertion failed: 2 != 3
#show emph: _ => test(query(heading).len(), 3)
#show strong: _ => test(query(heading).len(), 2)
= Introduction
= Background
*Hi* _there_

--- context-assign-to-captured-variable ---
// Test error when captured variable is assigned to.
#let i = 0
// Error: 11-12 variables from outside the context expression are read-only and cannot be modified
#context (i = 1)

--- context-compatibility-locate ---
#let s = state("x", 0)
#let compute(expr) = [
  #s.update(x =>
    eval(expr.replace("x", str(x)))
  )
  // Warning: 17-28 `state.display` is deprecated
  // Hint: 17-28 use `state.get` in a `context` expression instead
  New value is #s.display().
]

// Warning: 1:2-6:3 `locate` with callback function is deprecated
// Hint: 1:2-6:3 use a `context` expression instead
#locate(loc => {
  // Warning: 14-32 calling `query` with a location is deprecated
  // Hint: 14-32 try removing the location argument
  let elem = query(<here>, loc).first()
  test(s.at(elem.location()), 13)
})

#compute("10") \
#compute("x + 3") \
*Here.* <here> \
#compute("x * 2") \
#compute("x - 5")

--- context-compatibility-styling ---
// Warning: 2-53 `style` is deprecated
// Hint: 2-53 use a `context` expression instead
// Warning: 18-39 calling `measure` with a styles argument is deprecated
// Hint: 18-39 try removing the styles argument
#style(styles => measure([it], styles).width < 20pt)

--- context-compatibility-counter-display ---
#counter(heading).update(10)

// Warning: 2-44 `counter.display` without context is deprecated
// Hint: 2-44 use it in a `context` expression instead
#counter(heading).display(n => test(n, 10))

--- context-delayed-warning ---
// Ensure that the warning that triggers in the first layout iteration is not
// surfaced since it goes away in the second one. Just like errors in show
// rules.
#show heading: none

= A <a>
#context {
  let n = query(<a>).len()
  let fonts = ("nope", "Roboto")
  set text(font: fonts.at(n))
}
