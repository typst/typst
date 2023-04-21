// Test diagnostics.
// Ref: false

---
// Error: 1:17-1:19 expected length, found integer: a length needs a unit – did you mean 12pt?
#set text(size: 12)

---
#{
    let a = 2
    a = 1-a
    a = a -1

    // Error: 9-12 unknown variable: a-1 – did you mean a - 1?
    a = a-1
}
