// Test function calls.

---
// One argument.
#f(bold)

// One argument and trailing comma.
#f(1,)

// One named argument.
#f(a:2)

// Mixed arguments.
{f(1, a: (3, 4), 2, b: "5")}

---
// Different forms of template arguments.
// Ref: true

#let a = "a"

#f[a] \
#f(a) \
#f(a, [b]) \
#f(a)[b] \

// Template can be argument or body depending on whitespace.
#if "template" == type[b] [Sure ]
#if "template" == type [Nope.] #else [thing.]

// Should output `<function f> (Okay.)`.
#f (Okay.)

---
// Call function assigned to variable.
#let alias = type
#test(alias(alias), "function")

// Library function `font` returns template.
#test(type(font(12pt)), "template")
