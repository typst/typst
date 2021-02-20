// Test function calls.

---
// One argument.
#args(bold)

// One argument and trailing comma.
#args(1,)

// One named argument.
#args(a:2)

// Mixed arguments.
{args(1, b: "2", 3)}

---
// Different forms of template arguments.
// Ref: true

#let a = "a"

#args[a] \
#args(a) \
#args(a, [b]) \
#args(a)[b] \

// Template can be argument or body depending on whitespace.
#if "template" == type[b] [Sure ]
#if "template" == type [Nope.] #else [thing.]

// Should output `<function args> (Okay.)`.
#args (Okay.)

---
// Call function assigned to variable.
#let alias = type
#test(alias(alias), "function")

// Library function `font` returns template.
#test(type(font(12pt)), "template")
