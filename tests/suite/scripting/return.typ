// Test return out of functions.

--- return-with-value ---
// Test return with value.
#let f(x) = {
  return x + 1
}

#test(f(1), 2)

--- return-join ---
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

--- return-in-nested-content-block ---
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

--- return-outside-of-function ---
// Test return outside of function.

#for x in range(5) {
  // Error: 3-9 cannot return outside of function
  return
}

--- return-in-first-arg ---
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

--- return-in-content-block ---
// Test value return from content.
#let x = 3
#let f() = [
  Hello 😀
  #return "nope"
  World
]

#test(f(), "nope")

--- return-semicolon-or-linebreak ---
// Test rejection of extra value
#let f() = [
  // Error: 16-16 expected semicolon or line break
  #return a + b Hello World
]

--- return-discard-content ---
// Test that discarding joined content is a warning.

#let f() = {
  [ Hello, World! ]
  // Warning: 3-16 this return unconditionally discards the content before it
  // Hint: 3-16 use `return` without a value to return the joined content
  // Hint: 3-16 or try omitting the `return` keyword to also join this value
  return "nope"
}

#test(f(), "nope")

--- return-discard-content-nested ---

#let f() = {
  [ Hello, World! ]
  {
    // Warning: 5-18 this return unconditionally discards the content before it
    // Hint: 5-18 use `return` without a value to return the joined content
    // Hint: 5-18 or try omitting the `return` keyword to also join this value
    return "nope"
  }
}

#test(f(), "nope")

--- return-discard-for ---

#let f() = {
  state("hello").update("world")
  for x in range(3) {
    return "nope"
  }
}

#test(f(), "nope")

--- return-discard-while ---

#let f() = {
  state("hello").update("world")
  let i = 0
  while i < 10 {
    return "nope"
  }
}

#test(f(), "nope")

--- return-discard-state ---
// Test that discarding a joined content with state is special warning

#let f() = {
  state("hello").update("world")

  // Warning: 3-19 this return unconditionally discards the content before it
  // Hint: 3-19 it discards state and/or counter updates
  // Hint: 3-19 use `return` without a value to return the joined content
  // Hint: 3-19 or try omitting the `return` keyword to also join this value
  return [ Hello ]
}

#test(f(), [ Hello ])

--- return-no-discard ---
// Test that returning a joined content is not a warning.

#let f() = {
  state("hello").update("world")
  return
}

#test(f(), state("hello").update("world"))

--- return-discard-not-content ---
// Test that non-content joined value is not a warning.

#let f() = {
  (33,)
  return (66,)
}

#test(f(), (66, ))

--- return-discard-markup ---
// Test that discarding markup is not a warning.

#let f() = [
  hello
  #return [nope]
]

#test(f(), [nope])
