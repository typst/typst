// Test state.

--- state-basic ---
#let s = state("hey", "a")
#let double(it) = 2 * it

#s.update(double)
#s.update(double)
$ 2 + 3 $
#s.update(double)

Is: #context s.get(),
Was: #context {
  let it = query(math.equation).first()
  s.at(it.location())
}.

--- state-multiple-calls-same-key ---
// Try same key with different initial value.
#context state("key", 2).get()
#state("key").update(x => x + 1)
#context state("key", 2).get()
#context state("key", 3).get()
#state("key").update(x => x + 1)
#context state("key", 2).get()

--- state-nested ---
#set page(width: 200pt)
#set text(8pt)

#let ls = state("lorem", lorem(30).split(" "))
#let loremum(count) = {
  context ls.get().slice(0, count).join(".").trim() + "."
  ls.update(list => list.slice(count))
}

#let fs = state("fader", red)
#let trait(title) = block[
  #context text(fill: fs.get())[
    *#title:* #loremum(1)
  ]
  #fs.update(color => color.lighten(30%))
]

#trait[Boldness]
#trait[Adventure]
#trait[Fear]
#trait[Anger]

--- state-no-convergence ---
// Make sure that a warning is produced if the layout fails to converge.
// Warning: layout did not converge within 5 attempts
// Hint: check if any states or queries are updating themselves
#let s = state("s", 1)
#context s.update(s.final() + 1)
#context s.get()

--- state-at-no-context ---
// Test `state.at` outside of context.
// Error: 2-26 can only be used when context is known
// Hint: 2-26 try wrapping this in a `context` expression
// Hint: 2-26 the `context` expression should wrap everything that depends on this function
#state("key").at(<label>)
