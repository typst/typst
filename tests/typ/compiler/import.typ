// Test module imports.
// Ref: false

---
// Test basic syntax and semantics.
// Ref: true

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
#import "module.typ": fn, value
#fn[Like and Subscribe!]
#value

// Should output `bye`.
// Stop at semicolon.
#import "module.typ": a, c;bye

---
// An item import.
#import "module.typ": item
#test(item(1, 2), 3)

// Code mode
{
  import "module.typ": b
  test(b, 1)
}

// A wildcard import.
#import "module.typ": *

// It exists now!
#test(d, 3)

---
// A module import without items.
#import "module.typ"
#test(module.b, 1)
#test((module.item)(1, 2), 3)

---
// Who needs whitespace anyways?
#import"module.typ":*

// Allow the trailing comma.
#import "module.typ": a, c,

---
// Error: 9-11 failed to load file (is a directory)
#import "": name

---
// Error: 9-20 file not found (searched at typ/compiler/lib/0.2.1)
#import "lib/0.2.1"

---
// Some non-text stuff.
// Error: 9-30 file is not valid utf-8
#import "../../res/rhino.png"

---
// Unresolved import.
// Error: 23-35 unresolved import
#import "module.typ": non_existing

---
// Cyclic import of this very file.
// Error: 9-23 cyclic import
#import "./import.typ"

---
// Cyclic import in other file.
#import "./modules/cycle1.typ": *

This is never reached.

---
// Error: 8 expected expression
#import

---
// Error: 26-29 expected identifier, found string
#import "module.typ": a, "b", c

---
// Error: 22 expected import items
#import "module.typ":

---
// Error: 23-24 expected expression, found assignment operator
// Error: 24 expected import items
#import "module.typ": =

---
// An additional trailing comma.
// Error: 31-32 expected expression, found comma
#import "module.typ": a, b, c,,

---
// Error: 2:2 expected semicolon or line break
#import "module.typ
"stuff

---
// A star in the list.
// Error: 26-27 expected expression, found star
#import "module.typ": a, *, b

---
// An item after a star.
// Error: 24 expected semicolon or line break
#import "module.typ": *, a

---
// Error: 13-17 expected identifier, found named pair
#import "": a: 1
