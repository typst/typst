// Test import statements.

---
// Test importing semantics.

// A named import.
#import item from "target.typ"
#test(item(1, 2), 3)

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
#import fn, value from "target.typ"
#fn[Like and Subscribe!]
#value

// Code mode
{
  import b from "target.typ"
  test(b, 1)
}

// A wildcard import.
#import * from "target.typ"

// It exists now!
#d

// Who needs whitespace anyways?
#import*from"target.typ"

// Should output `bye`.
// Stop at semicolon.
#import a, c from "target.typ";bye

// Allow the trailing comma.
#import a, c, from "target.typ"

---
// Error: 19-21 file not found (searched at typ/code)
#import name from ""

---
// Error: 16-27 file not found (searched at typ/code/lib/0.2.1)
#import * from "lib/0.2.1"

---
// Some non-text stuff.
// Error: 16-37 failed to load source file (file is not valid utf-8)
#import * from "../../res/rhino.png"

---
// Unresolved import.
// Error: 9-21 unresolved import
#import non_existing from "target.typ"

---
// Cyclic import of this very file.
// Error: 16-30 cyclic import
#import * from "./import.typ"

---
// Cyclic import in other file.
#import * from "./importable/cycle1.typ"

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
#import from "target.typ"

// Error: 9-10 expected expression, found assignment operator
// Error: 10 expected import items
#import = from "target.typ"

// Error: 15 expected expression
#import * from

// An additional trailing comma.
// Error: 17-18 expected expression, found comma
#import a, b, c,, from "target.typ"

// Error: 1-6 unexpected keyword `from`
#from "target.typ"

// Error: 2:2 expected semicolon or line break
#import * from "target.typ
"target

// Error: 28 expected semicolon or line break
#import * from "target.typ" § 0.2.1

// A star in the list.
// Error: 12-13 expected expression, found star
#import a, *, b from "target.typ"

// An item after a star.
// Error: 10 expected keyword `from`
#import *, a from "target.typ"

---
// Error: 9-13 expected identifier, found named pair
#import a: 1 from ""
