// Test function and module imports.
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
// A renamed item import.
#import "module.typ": item as something
#test(something(1, 2), 3)

// Mixing renamed and not renamed items.
#import "module.typ": fn, b as val, item as other
#test(val, 1)
#test(other(1, 2), 3)

---
// Test importing from function scopes.
// Ref: true

#import enum: item
#import assert.with(true): *

#enum(
   item(1)[First],
   item(5)[Fifth]
)
#eq(10, 10)
#ne(5, 6)

---
// Test renaming items imported from function scopes.
#import assert: eq as aseq
#aseq(10, 10)

---
// A module import without items.
#import "module.typ"
#test(module.b, 1)
#test(module.item(1, 2), 3)
#test(module.push(2), 3)

---
// A renamed module import without items.
#import "module.typ" as other
#test(other.b, 1)
#test(other.item(1, 2), 3)
#test(other.push(2), 3)

---
// Mixing renamed module and items.
#import "module.typ" as newname: b as newval, item
#test(newname.b, 1)
#test(newval, 1)
#test(item(1, 2), 3)
#test(newname.item(1, 2), 3)

---
// Renamed module import with function scopes.
#import enum as othernum
#test(enum, othernum)

---
// Mixing renamed module import from function with renamed item import.
#import assert as asrt
#import asrt: ne as asne
#asne(1, 2)

---
// Edge case for module access that isn't fixed.
#import "module.typ"

// Works because the method name isn't categorized as mutating.
#test((module,).at(0).item(1, 2), 3)

// Doesn't work because of mutating name.
// Error: 2-11 cannot mutate a temporary value
#(module,).at(0).push()

---
// Who needs whitespace anyways?
#import"module.typ":*

// Allow the trailing comma.
#import "module.typ": a, c,

---
// Usual importing syntax also works for function scopes
#let d = (e: enum)
#import d.e
#import d.e as renamed
#import d.e: item
#item(2)[a]

---
// Warning: 23-27 unnecessary import rename to same name
#import enum: item as item

---
// Warning: 17-21 unnecessary import rename to same name
#import enum as enum

---
// Warning: 17-21 unnecessary import rename to same name
#import enum as enum: item
// Warning: 17-21 unnecessary import rename to same name
// Warning: 31-35 unnecessary import rename to same name
#import enum as enum: item as item

---
// No warning on a case that isn't obviously pathological
#import "module.typ" as module

---
// Can't import from closures.
#let f(x) = x
// Error: 9-10 cannot import from user-defined functions
#import f: x

---
// Can't import from closures, despite renaming.
#let f(x) = x
// Error: 9-10 cannot import from user-defined functions
#import f as g

---
// Can't import from closures, despite modifiers.
#let f(x) = x
// Error: 9-18 cannot import from user-defined functions
#import f.with(5): x

---
// Error: 9-18 cannot import from user-defined functions
#import () => {5}: x

---
// Error: 9-10 expected path, module, function, or type, found integer
#import 5: something

---
// Error: 9-10 expected path, module, function, or type, found integer
#import 5 as x

---
// Error: 9-11 failed to load file (is a directory)
#import "": name

---
// Error: 9-11 failed to load file (is a directory)
#import "" as x

---
// Error: 9-20 file not found (searched at typ/compiler/lib/0.2.1)
#import "lib/0.2.1"

---
// Error: 9-20 file not found (searched at typ/compiler/lib/0.2.1)
#import "lib/0.2.1" as x

---
// Some non-text stuff.
// Error: 9-27 file is not valid utf-8
#import "/files/rhino.png"

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
// Renaming does not import the old name (without items).
#import "module.typ" as something
// Error: 7-12 unknown variable: mymod
#test(mymod.b, 1)

---
// Renaming does not import the old name (with items).
#import "module.typ" as something: b as other
// Error: 7-12 unknown variable: mymod
#test(mymod.b, 1)

---
// Error: 8 expected expression
#import

---
// Error: 26-29 unexpected string
#import "module.typ": a, "b", c

---
// Error: 23-24 unexpected equals sign
#import "module.typ": =

---
// An additional trailing comma.
// Error: 31-32 unexpected comma
#import "module.typ": a, b, c,,

---
// Error: 2:2 expected semicolon or line break
#import "module.typ
"stuff

---
// A star in the list.
// Error: 26-27 unexpected star
#import "module.typ": a, *, b

---
// An item after a star.
// Error: 24 expected semicolon or line break
#import "module.typ": *, a

---
// Error: 14-15 unexpected colon
// Error: 16-17 unexpected integer
#import "": a: 1

---
// Error: 14 expected comma
#import "": a b
