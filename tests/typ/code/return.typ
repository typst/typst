// Test return out of functions.
// Ref: false

---
// Test return with value.
#let f(x) = {
  return x + 1
}

#test(f(1), 2)

---
// Test return with joining.

#let f(x) = {
  "a"
  if x == 0 {
    return "b"
  } else if x == 1 {
    "c"
  } else {
    "d"
    return
    "e"
  }
}

#test(f(0), "b")
#test(f(1), "ac")
#test(f(2), "ad")

---
// Test return with joining and content.
// Ref: true

#let f(text, caption: none) = {
  text
  if caption == none {
    [\.]
    return
  }
  [, ]
  emph(caption)
  [\.]
}

#f(caption: [with caption])[My figure]

#f[My other figure]

---
// Test return outside of function.

#for x in range(5) {
  // Error: 3-9 cannot return outside of function
  return
}
