// Test return value of if expressions.
// Ref: false

---
{
    let x = 1
    let y = 2
    let z

    // Returns if branch.
    z = if x < y { "ok" }
    test(z, "ok")

    // Returns else branch.
    z = if x > y { "bad" } else { "ok" }
    test(z, "ok")

    // Missing else evaluates to none.
    z = if x > y { "bad" }
    test(z, none)
}
