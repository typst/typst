// Test function and module imports.

--- import-basic ---
// Test basic syntax and semantics.

// Test that this will be overwritten.
#let value = [foo]

// Import multiple things.
#import "module.typ": fn, value
#fn[Like and Subscribe!]
#value

// Should output `bye`.
// Stop at semicolon.
#import "module.typ": a, c;bye

--- import-item-markup ---
// An item import.
#import "module.typ": item
#test(item(1, 2), 3)

--- import-item-in-code ---
// Code mode
#{
  import "module.typ": b
  test(b, 1)
}

--- import-wildcard-in-markup ---
// A wildcard import.
#import "module.typ": *

// It exists now!
#test(d, 3)

--- import-item-renamed ---
// A renamed item import.
#import "module.typ": item as something
#test(something(1, 2), 3)

--- import-items-renamed-mixed ---
// Mixing renamed and not renamed items.
#import "module.typ": fn, b as val, item as other
#test(val, 1)
#test(other(1, 2), 3)

--- import-nested-item ---
// Nested item imports.
#import "modules/chap1.typ" as orig-chap1
#import "modules/chap2.typ" as orig-chap2
#import "module.typ": chap2, chap2.name, chap2.chap1, chap2.chap1.name as othername
#test(chap2, orig-chap2)
#test(chap1, orig-chap1)
#test(name, "Peter")
#test(othername, "Klaus")

--- import-items-parenthesized ---
#import "module.typ": ()
#import "module.typ": (a)
#import "module.typ": (a, b)
#import "module.typ": (a, b, c, d)

#test(a, none)
#test(b, 1)
#test(c, 2)
#test(d, 3)

--- import-items-parenthesized-multiline ---
#import "module.typ": (
  a
)
#import "module.typ": (
  a, b as e,
  c,


      d,
)

#test(a, none)
#test(e, 1)
#test(c, 2)
#test(d, 3)

--- import-items-parenthesized-invalid ---
// Error: 23-24 unclosed delimiter
#import "module.typ": (a, b, c

--- import-items-parenthesized-invalid-2 ---
// Error: 23-24 unclosed delimiter
#import "module.typ": (

--- import-items-parenthesized-invalid-3 ---
// Error: 23-24 unclosed delimiter
#import "module.typ": (
  a, b,
  c,


--- import-from-function-scope ---
// Test importing from function scopes.

#import enum: item
#import assert.with(true): *

#enum(
   item(1)[First],
   item(5)[Fifth]
)
#eq(10, 10)
#ne(5, 6)

--- import-from-function-scope-item-renamed ---
// Test renaming items imported from function scopes.
#import assert: eq as aseq
#aseq(10, 10)

--- import-from-function-scope-nested-import ---
// Test importing items from function scopes via nested import.
#import std: grid.cell, table.cell as tcell
#test(cell, grid.cell)
#test(tcell, table.cell)

--- import-from-type-scope ---
// Test importing from a type's scope.
#import array: zip
#test(zip((1, 2), (3, 4)), ((1, 3), (2, 4)))

--- import-from-type-scope-item-renamed ---
// Test importing from a type's scope with renaming.
#import array: pop as renamed-pop
#test(renamed-pop((1, 2)), 2)

--- import-from-type-scope-nested-import ---
// Test importing from a type's scope with nested import.
#import std: array.zip, array.pop as renamed-pop
#test(zip((1, 2), (3, 4)), ((1, 3), (2, 4)))
#test(renamed-pop((1, 2)), 2)

--- import-from-file-bare ---
// A module import without items.
#import "module.typ"
#test(module.b, 1)
#test(module.item(1, 2), 3)
#test(module.push(2), 3)

--- import-from-file-bare-invalid ---
// Error: 9-33 module name would not be a valid identifier
// Hint: 9-33 you can rename the import with `as`
#import "modules/with space.typ"

--- import-from-file-bare-dynamic ---
// Error: 9-26 dynamic import requires an explicit name
// Hint: 9-26 you can name the import with `as`
#import "mod" + "ule.typ"

--- import-from-var-bare ---
#let p = "module.typ"
// Error: 9-10 dynamic import requires an explicit name
// Hint: 9-10 you can name the import with `as`
#import p
#test(p.b, 1)

--- import-from-dict-field-bare ---
#let d = (p: "module.typ")
// Error: 9-12 dynamic import requires an explicit name
// Hint: 9-12 you can name the import with `as`
#import d.p
#test(p.b, 1)

--- import-from-file-renamed-dynamic ---
#import "mod" + "ule.typ" as mod
#test(mod.b, 1)

--- import-from-file-renamed ---
// A renamed module import without items.
#import "module.typ" as other
#test(other.b, 1)
#test(other.item(1, 2), 3)
#test(other.push(2), 3)

--- import-from-file-items-renamed-mixed ---
// Mixing renamed module and items.
#import "module.typ" as newname: b as newval, item
#test(newname.b, 1)
#test(newval, 1)
#test(item(1, 2), 3)
#test(newname.item(1, 2), 3)

--- import-from-function-scope-bare ---
// Warning: 9-13 this import has no effect
#import enum

--- import-from-function-scope-renamed ---
// Renamed module import with function scopes.
#import enum as othernum
#test(enum, othernum)

--- import-from-function-scope-renamed-twice ---
// Mixing renamed module import from function with renamed item import.
#import assert as asrt
#import asrt: ne as asne
#asne(1, 2)

--- import-from-module-bare ---
#import "modules/chap1.typ" as mymod
// Warning: 9-14 this import has no effect
#import mymod
// The name `chap1` is not bound.
// Error: 2-7 unknown variable: chap1
#chap1

--- import-module-nested ---
#import std.calc: pi
#test(pi, calc.pi)

--- import-module-nested-bare ---
#import "module.typ"
#import module.chap2
#test(chap2.name, "Peter")

--- import-module-item-name-mutating ---
// Edge case for module access that isn't fixed.
#import "module.typ"

// Works because the method name isn't categorized as mutating.
#test((module,).at(0).item(1, 2), 3)

// Doesn't work because of mutating name.
// Error: 2-11 cannot mutate a temporary value
#(module,).at(0).push()

--- import-no-whitespace ---
// Who needs whitespace anyways?
#import"module.typ":*

--- import-trailing-comma ---
// Allow the trailing comma.
#import "module.typ": a, c,

--- import-source-field-access ---
// Usual importing syntax also works for function scopes
#let d = (e: enum)
#import d.e
#import d.e as renamed
#import d.e: item
#item(2)[a]

--- import-item-rename-unnecessary ---
// Warning: 23-27 unnecessary import rename to same name
#import enum: item as item

--- import-rename-unnecessary ---
// Warning: 17-21 unnecessary import rename to same name
#import enum as enum

--- import-rename-necessary ---
#import "module.typ" as module: a
#test(module.a, a)

--- import-rename-unnecessary-mixed ---
// Warning: 17-21 unnecessary import rename to same name
#import enum as enum: item

// Warning: 17-21 unnecessary import rename to same name
// Warning: 31-35 unnecessary import rename to same name
#import enum as enum: item as item

--- import-item-rename-unnecessary-but-ok ---
#import "modul" + "e.typ" as module
#test(module.b, 1)

--- import-from-closure-invalid ---
// Can't import from closures.
#let f(x) = x
// Error: 9-10 cannot import from user-defined functions
#import f: x

--- import-from-closure-renamed-invalid ---
// Can't import from closures, despite renaming.
#let f(x) = x
// Error: 9-10 cannot import from user-defined functions
#import f as g

--- import-from-with-closure-invalid ---
// Can't import from closures, despite modifiers.
#let f(x) = x
// Error: 9-18 cannot import from user-defined functions
#import f.with(5): x

--- import-from-with-closure-literal-invalid ---
// Error: 9-18 cannot import from user-defined functions
#import () => {5}: x

--- import-from-int-invalid ---
// Error: 9-10 expected path, module, function, or type, found integer
#import 5: something

--- import-from-int-renamed-invalid ---
// Error: 9-10 expected path, module, function, or type, found integer
#import 5 as x

--- import-from-string-invalid ---
// Error: 9-11 failed to load file (is a directory)
#import "": name

--- import-from-string-renamed-invalid ---
// Error: 9-11 failed to load file (is a directory)
#import "" as x

--- import-file-not-found-invalid ---
// Error: 9-20 file not found (searched at tests/suite/scripting/lib/0.2.1)
#import "lib/0.2.1"

--- import-file-not-found-renamed-invalid ---
// Error: 9-20 file not found (searched at tests/suite/scripting/lib/0.2.1)
#import "lib/0.2.1" as x

--- import-file-not-valid-utf-8 ---
// Some non-text stuff.
// Error: 9-35 file is not valid utf-8
#import "/assets/images/rhino.png"

--- import-item-not-found ---
// Unresolved import.
// Error: 23-35 unresolved import
#import "module.typ": non_existing

--- import-cyclic ---
// Cyclic import of this very file.
// Error: 9-23 cyclic import
#import "./import.typ"

--- import-cyclic-in-other-file ---
// Cyclic import in other file.
// Error: "tests/suite/scripting/modules/cycle2.typ" 2:9-2:21 cyclic import
#import "./modules/cycle1.typ": *

This is never reached.

--- import-renamed-old-name ---
// Renaming does not import the old name (without items).
#import "./modules/chap1.typ" as something
#test(something.name, "Klaus")
// Error: 7-12 unknown variable: chap1
#test(chap1.name, "Klaus")

--- import-items-renamed-old-name ---
// Renaming does not import the old name (with items).
#import "./modules/chap1.typ" as something: name as other
#test(other, "Klaus")
#test(something.name, "Klaus")
// Error: 7-12 unknown variable: chap1
#test(chap1.b, "Klaus")

--- import-nested-invalid-type ---
// Error: 19-21 expected module, function, or type, found float
#import std: calc.pi.something

--- import-incomplete ---
// Error: 8 expected expression
#import

--- import-item-string-invalid ---
// Error: 26-29 unexpected string
#import "module.typ": a, "b", c

--- import-bad-token ---
// Error: 23-24 unexpected equals sign
#import "module.typ": =

--- import-duplicate-comma ---
// An additional trailing comma.
// Error: 31-32 unexpected comma
#import "module.typ": a, b, c,,

--- import-no-colon ---
// Error: 2:2 expected semicolon or line break
#import "module.typ
"stuff

--- import-bad-token-star ---
// A star in the list.
// Error: 26-27 unexpected star
#import "module.typ": a, *, b

--- import-item-after-star ---
// An item after a star.
// Error: 24 expected semicolon or line break
#import "module.typ": *, a

--- import-bad-colon-in-items ---
// Error: 14-15 unexpected colon
// Error: 16-17 unexpected integer
#import "": a: 1

--- import-incomplete-nested ---
// Error: 15 expected identifier
#import "": a.

--- import-wildcard-in-nested ---
// Error: 15 expected identifier
// Error: 15-16 unexpected star
#import "": a.*

--- import-missing-comma ---
// Error: 14 expected comma
#import "": a b

--- import-from-package-bare ---
// Test import without items.
#import "@test/adder:0.1.0"
#test(adder.add(2, 8), 10)

--- import-from-package-dynamic ---
// Error: 9-33 dynamic import requires an explicit name
// Hint: 9-33 you can name the import with `as`
#import "@test/" + "adder:0.1.0"

--- import-from-package-renamed-dynamic ---
#import "@test/" + "adder:0.1.0" as adder
#test(adder.add(2, 8), 10)

--- import-from-package-items ---
// Test import with items.
#import "@test/adder:0.1.0": add
#test(add(2, 8), 10)

--- import-from-package-required-compiler-version ---
// Test too high required compiler version.
// Error: 9-29 package requires typst 1.0.0 or newer (current version is VERSION)
#import "@test/future:0.1.0": future

--- import-from-package-namespace-invalid-1 ---
// Error: 9-13 `@` is not a valid package namespace
#import "@@": *

--- import-from-package-name-missing-1 ---
// Error: 9-16 package specification is missing name
#import "@heya": *

--- import-from-package-namespace-invalid-2 ---
// Error: 9-15 `123` is not a valid package namespace
#import "@123": *

--- import-from-package-name-missing-2 ---
// Error: 9-17 package specification is missing name
#import "@test/": *

--- import-from-package-version-missing-1 ---
// Error: 9-22 package specification is missing version
#import "@test/mypkg": *

--- import-from-package-name-invalid ---
// Error: 9-20 `$$$` is not a valid package name
#import "@test/$$$": *

--- import-from-package-version-missing-2 ---
// Error: 9-23 package specification is missing version
#import "@test/mypkg:": *

--- import-from-package-version-missing-minor ---
// Error: 9-24 version number is missing minor version
#import "@test/mypkg:0": *

--- import-from-package-version-major-invalid-1 ---
// Error: 9-29 `latest` is not a valid major version
#import "@test/mypkg:latest": *

--- import-from-package-version-major-invalid-2 ---
// Error: 9-29 `-3` is not a valid major version
#import "@test/mypkg:-3.0.0": *

--- import-from-package-version-missing-patch-1 ---
// Error: 9-26 version number is missing patch version
#import "@test/mypkg:0.3": *

--- import-from-package-version-missing-patch-2 ---
// Error: 9-27 version number is missing patch version
#import "@test/mypkg:0.3.": *

--- import-from-file-package-lookalike ---
// Error: 9-28 file not found (searched at tests/suite/scripting/#test/mypkg:1.0.0)
#import "#test/mypkg:1.0.0": *
