// Test closures.
// Ref: false

---

// Basic closure without captures.
{
    let adder = (x, y) => x + y
    test(adder(2, 3), 5)
}

// Pass closure as argument and return closure.
// Also uses shorthand syntax for a single argument.
{
    let chain = (f, g) => (x) => f(g(x))
    let f = x => x + 1
    let g = x => 2 * x
    let h = chain(f, g)
    test(h(2), 5)
}

// Capture environment.
{
    let mark = "?"
    let greet = {
        let hi = "Hi"
        name => {
            hi + ", " + name + mark
        }
    }

    test(greet("Typst"), "Hi, Typst?")

    mark = "!"
    test(greet("Typst"), "Hi, Typst!")
}

// Don't leak environment.
{
    // Error: 18-19 unknown variable
    let func() = x
    let x = "hi"

    test(func(), error)
}

// Redefined variable.
{
    let x = 1
    let f() = {
        let x = x + 2
        x
    }
    test(f(), 3)
}

---
// Too few arguments.
{
    let types(x, y) = "[" + type(x) + ", " + type(y) + "]"
    test(types(14%, 12pt), "[relative, length]")

    // Error: 16-22 missing argument: y
    test(types("nope"), "[string, none]")
}

// Too many arguments.
{
    let f(x) = x + 1

    // Error: 2:10-2:15 unexpected argument
    // Error: 1:17-1:24 unexpected argument
    f(1, "two", () => x)
}
