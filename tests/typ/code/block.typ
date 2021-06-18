// Test code blocks.

---
All none

// Nothing evaluates to none.
{}

// Let evaluates to none.
{ let v = 0 }

// Type is joined with trailing none, evaluates to string.
{
    type("")
    none
}

---
// Evaluates to single expression.
{ "Hello" }

// Evaluates to string.
{ let x = "Hel"; x + "lo" }

// Evaluates to join of none, [He] and the two loop bodies.
{
    let parts = ("l", "lo")
    [He]
    for s in parts [{s}]
}

---
// Evaluates to join of the templates and strings.
{
    [Hey, ]
    if true {
        "there!"
    }
    [ ]
    if false [Nope]
    [How are ] + "you?"
}

{
    [A]
    // Error: 5-6 cannot join template with integer
    1
    [B]
}

---
// Works the same way in code environment.
// Ref: false
#test(3, {
    let x = 1
    let y = 2
    x + y
})
