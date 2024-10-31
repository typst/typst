--- fold-vec-order-text-features ---
// Test fold order of vectors.
#set text(features: (liga: 1))
#set text(features: (liga: 0))
fi

--- fold-vec-order-text-decos ---
#underline(stroke: aqua + 4pt)[
  #underline[Hello]
]

--- fold-vec-order-meta ---
#let c = counter("mycounter")
#c.update(1)

// Warning: 1:2-7:3 `locate` with callback function is deprecated
// Hint: 1:2-7:3 use a `context` expression instead
#locate(loc => [
  #c.update(2)
  #c.at(loc) \
  // Warning: 12-36 `locate` with callback function is deprecated
  // Hint: 12-36 use a `context` expression instead
  Second: #locate(loc => c.at(loc))
])
