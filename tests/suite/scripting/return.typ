// Test return out of functions.

--- return-with-value paged ---
// Test return with value.
#let f(x) = {
  return x + 1
}

#test(f(1), 2)

--- return-join paged ---
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

--- return-in-nested-content-block paged ---
// Test return with joining and content.

#let f(text, caption: none) = {
  text
  if caption == none [\.#return]
  [, ]
  emph(caption)
  [\.]
}

#f(caption: [with caption])[My figure]

#f[My other figure]

--- return-outside-of-function paged ---
// Test return outside of function.

#for x in range(5) {
  // Error: 3-9 cannot return outside of function
  return
}

--- return-in-first-arg paged ---
// Test that the expression is evaluated to the end.
#let sum(..args) = {
  let s = 0
  for v in args.pos() {
    s += v
  }
  s
}

#let f() = {
  sum(..return, 1, 2, 3)
  "nope"
}

#test(f(), 6)

--- return-in-content-block paged ---
// Test value return from content.
#let x = 3
#let f() = [
  Hello 😀
  #return "nope"
  World
]

#test(f(), "nope")

--- return-semicolon-or-linebreak paged ---
// Test rejection of extra value
#let f() = [
  // Error: 16-16 expected semicolon or line break
  #return a + b Hello World
]

--- return-discard-content paged ---
// Test that discarding joined content is a warning.
#let f() = {
  [Hello, World!]
  // Warning: 3-16 this return unconditionally discards the content before it
  // Hint: 3-16 try omitting the `return` to automatically join all values
  return "nope"
}

#test(f(), "nope")

--- return-discard-content-nested paged ---
#let f() = {
  [Hello, World!]
  {
    // Warning: 5-18 this return unconditionally discards the content before it
    // Hint: 5-18 try omitting the `return` to automatically join all values
    return "nope"
  }
}

#test(f(), "nope")

--- return-discard-state paged ---
// Test that discarding a joined content with state is special warning

#let f() = {
  state("hello").update("world")

  // Warning: 3-19 this return unconditionally discards the content before it
  // Hint: 3-19 try omitting the `return` to automatically join all values
  // Hint: 3-19 state/counter updates are content that must end up in the document to have an effect
  return [ Hello ]
}

#test(f(), [ Hello ])

--- return-discard-loop paged ---
// Test that return from within a control flow construct is not a warning.
#let f1() = {
  state("hello").update("world")
  for x in range(3) {
    return "nope1"
  }
}

#let f2() = {
  state("hello").update("world")
  let i = 0
  while i < 10 {
    return "nope2"
  }
}

#test(f1(), "nope1")
#test(f2(), "nope2")

--- return-no-discard paged ---
// Test that returning the joined content is not a warning.
#let f() = {
  state("hello").update("world")
  return
}

#test(f(), state("hello").update("world"))

--- return-discard-not-content paged ---
// Test that non-content joined value is not a warning.
#let f() = {
  (33,)
  return (66,)
}

#test(f(), (66, ))

--- return-discard-markup paged ---
// Test that discarding markup is not a warning.
#let f() = [
  hello
  #return [nope]
]

#test(f(), [nope])
