// Test invalid code block syntax.

---
// Multiple unseparated expressions in one line.

// Error: 2-4 expected expression, found invalid token
{1u}

// Should output `1`.
// Error: 3-3 expected semicolon or line break
{0 1}

// Should output `2`.
// Error: 2:13-2:13 expected semicolon or line break
// Error: 1:24-1:24 expected semicolon or line break
{#let x = -1 #let y = 3 x + y}

// Should output `3`.
{
    // Error: 10-13 expected identifier, found string
    #for "v"

    // Error: 11-11 expected keyword `#in`
    #for v #let z = 1 + 2

    z
}

---
// Ref: false
// Error: 3:1-3:1 expected closing brace
{

---
// Ref: false
// Error: 1-2 unexpected closing brace
}
