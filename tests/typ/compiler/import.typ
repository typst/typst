// Test module imports.

---
// Test importing semantics.

// A named import.
#import item from "module.typ"
#test(item(1, 2), 3)

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
#import fn, value from "module.typ"
#fn[Like and Subscribe!]
#value

// Code mode
{
  import b from "module.typ"
  test(b, 1)
}

// A wildcard import.
#import * from "module.typ"

// It exists now!
#d

// Who needs whitespace anyways?
#import*from"module.typ"

// Should output `bye`.
// Stop at semicolon.
#import a, c from "module.typ";bye

// Allow the trailing comma.
#import a, c, from "module.typ"

---
// Error: 19-21 failed to load file (is a directory)
#import name from ""

---
// Error: 16-27 file not found (searched at typ/compiler/lib/0.2.1)
#import * from "lib/0.2.1"

---
// Some non-text stuff.
// Error: 16-37 file is not valid utf-8
#import * from "../../res/rhino.png"

---
// Unresolved import.
// Error: 9-21 unresolved import
#import non_existing from "module.typ"

---
// Cyclic import of this very file.
// Error: 16-30 cyclic import
#import * from "./import.typ"

---
// Cyclic import in other file.
#import * from "./modules/cycle1.typ"

This is never reached.

---
// Error: 8 expected import items
// Error: 8 expected keyword `from`
#import

// Error: 9-19 expected identifier, found string
// Error: 19 expected keyword `from`
#import "file.typ"

// Error: 16-19 expected identifier, found string
// Error: 22 expected keyword `from`
#import afrom, "b", c

// Error: 9 expected import items
#import from "module.typ"

// Error: 9-10 expected expression, found assignment operator
// Error: 10 expected import items
#import = from "module.typ"

// Error: 15 expected expression
#import * from

// An additional trailing comma.
// Error: 17-18 expected expression, found comma
#import a, b, c,, from "module.typ"

// Error: 1-6 unexpected keyword `from`
#from "module.typ"

// Error: 2:2 expected semicolon or line break
#import * from "module.typ
"target

// Error: 28 expected semicolon or line break
#import * from "module.typ" § 0.2.1

// A star in the list.
// Error: 12-13 expected expression, found star
#import a, *, b from "module.typ"

// An item after a star.
// Error: 10 expected keyword `from`
#import *, a from "module.typ"

---
// Error: 9-13 expected identifier, found named pair
#import a: 1 from ""
