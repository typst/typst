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

// Should output `() + 2`.
#args() + 2

---
// Ref: false

// Call function assigned to variable.
#let alias = type
#test(alias(alias), "function")

// Library function `font` returns template.
#test(type(font(size: 12pt)), "template")

---
// Callee expressions.
{
    // Error: 5-9 expected function, found boolean
    true()

    // Wrapped in parens.
    test((type)("hi"), "string")

    // Call the return value of a function.
    let adder(dx) = x => x + dx
    test(adder(2)(5), 7)
}

#let f(x, body) = (y) => {
    [{x}] + body + [{y}]
}

// Call return value of function with body.
#f(1)[2](3)

// Don't allow this to be a closure.
// Should output `x => "hi"`.
#let x = "x"
#x => "hi"

---
// Different forms of template arguments.

#let a = "a"

#args(a) \
#args[a] \
#args(a, [b])

// Template can be argument or body depending on whitespace.
#if "template" == type[b] [Sure ]
#if "template" == type [Nope.] #else [thing.]

// Should output `<function args> (Okay.)`.
#args (Okay.)
