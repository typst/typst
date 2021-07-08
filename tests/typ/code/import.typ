// Test import statements.

---
// Test importing semantics.

// A named import.
#import item from "target.typ"
#test(item(1, 2), 3)

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
// Error: 9-10 expected expression, found comma
#import ,fn, value from "target.typ"
#fn[Like and Subscribe!]
#value

// Code mode
{
    import b from "target.typ"
    test(b, 1)
}

#test(b, 1)

// This should not exist yet
// Error: 1-3 unknown variable
#d

// A wildcard import.
#import * from "target.typ"

// It exists now!
#d

// Who needs whitespace anyways?
#import*from"target.typ"

// Should output `Hi`.
// Stop at semicolon.
#import a, c from "target.typ";bye

// Allow the trailing comma.
#import a, c, from "target.typ"

---
// Test bad imports.
// Ref: false

// Error: 19-21 file not found
#import name from ""

// Error: 16-27 file not found
#import * from "lib/0.2.1"

// Some non-text stuff.
// Error: 16-37 file is not valid utf-8
#import * from "../../res/rhino.png"

// Unresolved import.
// Error: 9-21 unresolved import
#import non_existing from "target.typ"

// Cyclic import.
// Error: 16-41 cyclic import
#import * from "./importable/cycle1.typ"

---
// Test bad syntax.

// Error: 2:8 expected import items
// Error: 1:8 expected keyword `from`
#import

// Error: 2:9-2:19 expected identifier
// Error: 1:19 expected keyword `from`
#import "file.typ"

// Error: 2:16-2:19 expected identifier
// Error: 1:22 expected keyword `from`
#import afrom, "b", c

// Error: 8 expected import items
#import from "target.typ"

// Error: 2:9-2:10 expected expression, found assignment operator
// Error: 1:10 expected import items
#import = from "target.typ"

// Error: 15 expected expression
#import * from

// An additional trailing comma.
// Error: 17-18 expected expression, found comma
#import a, b, c,, from "target.typ"

// Should output `"target.typ"`.
// Error: 1-6 unexpected keyword `from`
#from "target.typ"

// Should output `target`.
// Error: 2:16-3:2 file not found
// Error: 2:2 expected semicolon or line break
#import * from "target.typ
"target

// Should output `@ 0.2.1`.
// Error: 28 expected semicolon or line break
#import * from "target.typ" @ 0.2.1

// A star in the list.
// Error: 2:12-2:13 expected expression, found star
// Error: 1:13-1:14 expected expression, found comma
#import a, *, b from "target.typ"

// An item after a star.
// Should output `, a from "target.typ"`.
// Error: 10 expected keyword `from`
#import *, a from "target.typ"
