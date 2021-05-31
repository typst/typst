// Test import statements.

---
// Test importing semantics.

// A named import.
#import "target.typ" using item
#test(item(1, 2), 3)

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
// Error: 28-29 expected expression, found comma
#import "target.typ" using ,fn, value
#fn[Like and Subscribe!]
#value

// Code mode
{
    import "target.typ" using b
    test(b, 1)
}

#test(b, 1)

// This should not exist yet
// Error: 1-3 unknown variable
#d

// A wildcard import.
#import "target.typ" using *

// It exists now!
#d

---
// Test bad imports.
// Ref: false

// Error: 9-11 file not found
#import "" using name

// Error: 9-20 file not found
#import "lib/0.2.1" using *

// Error: 9-20 file not found
#import "lib@0.2.1" using *

// Some non-text stuff.
// Error: 9-30 file is not valid utf-8
#import "../../res/rhino.png" using *

// Unresolved import.
// Error: 28-40 unresolved import
#import "target.typ" using non_existing

// Cyclic import.
// Error: 9-34 cyclic import
#import "./importable/cycle1.typ" using *

---
// Test syntax.

// Missing file.
// Error: 9-10 expected expression, found star
#import *

// Should output `"target.typ"`.
// Error: 1-7 unexpected keyword `using`
#using "target.typ"

// Should output `target`.
// Error: 3:9-4:8 file not found
// Error: 3:8 expected semicolon or line break
// Error: 2:8 expected keyword `using`
#import "target.typ
using "target

// Should output `@ 0.2.1 using`.
// Error: 2:21 expected semicolon or line break
// Error: 1:21 expected keyword `using`
#import "target.typ" @ 0.2.1 using *

// Error: 3:21 expected keyword `using`
// Error: 2:21 expected semicolon or line break
// Error: 1:22-1:28 unexpected keyword `using`
#import "target.typ" #using *

// Error: 2:21 expected semicolon or line break
// Error: 1:21 expected keyword `using`
#import "target.typ" usinga,b,c

// Error: 27 expected import items
#import "target.typ" using

// Error: 2:28-2:29 expected expression, found assignment operator
// Error: 1:29 expected import items
#import "target.typ" using =

// Allow the trailing comma.
#import "target.typ" using a, c,

// An additional trailing comma.
// Error: 36-37 expected expression, found comma
#import "target.typ" using a, b, c,,

// Star in the list.
// Error: 2:31-2:32 expected expression, found star
// Error: 32-33 expected expression, found comma
#import "target.typ" using a, *, b

// Stop at semicolon.
#import "target.typ" using a, c;Hi

// Who needs whitespace anyways?
#import "target.typ"using *
#import"target.typ"using*
#import "target.typ"using *
