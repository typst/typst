--- fold-vec-order-text-features render ---
// Test fold order of vectors.
#set text(features: (liga: 1))
#set text(features: (liga: 0))
fi

--- fold-vec-order-text-decos render ---
#underline(stroke: aqua + 4pt)[
  #underline[Hello]
]

--- fold-vec-order-meta render ---
#let c = counter("mycounter")
#c.update(1)

#context [
  #c.update(2)
  #c.get() \
  Second: #context c.get()
]
