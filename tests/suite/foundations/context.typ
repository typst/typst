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

--- context-body-is-closure ---
// Regression test since this used to be a hard crash.
#(context (a: none) => {})
