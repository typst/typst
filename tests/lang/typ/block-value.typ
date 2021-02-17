// Test return value of code blocks.

---
All none

// Nothing evaluates to none.
{}

// Let evaluates to none.
{ let v = 0 }

// Trailing none evaluates to none.
{
    type("")
    none
}

---
// Evaluates to single expression.
{ "Hello" }

// Evaluates to trailing expression.
{ let x = "Hel"; x + "lo" }

// Evaluates to concatenation of for loop bodies.
{
    let parts = ("Hel", "lo")
    for s in parts [{s}]
}

---
// Works the same way in code environment.
// Ref: false
#test(3, {
    let x = 1
    let y = 2
    x + y
})
