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

    // Error: 9-12 unknown variable: a-1 – if you meant to use subtraction, try adding space around the minus sign.
    a = a-1
}

---
#{
    // Error: 5-8 unknown variable: a-1 – if you meant to use subtraction, try adding space around the minus sign.
    a-1 = 2
}
