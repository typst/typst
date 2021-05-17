// Test scoping with blocks.
// Ref: false

---
// Block in template does not create a scope.
{ let x = 1 }
#test(x, 1)

---
// Block in expression does create a scope.
#let a = {
    let b = 1
    b
}

#test(a, 1)

// Error: 2-3 unknown variable
{b}

---
// Multiple nested scopes.
{
    let a = "a1"
    {
        let a = "a2"
        {
            test(a, "a2")
            let a = "a3"
            test(a, "a3")
        }
        test(a, "a2")
    }
    test(a, "a1")
}
