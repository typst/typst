// Test invalid code block syntax.

---
// Multiple unseparated expressions in one line.

// Error: 2-4 expected expression, found invalid token
{1u}

// Should output `1`.
// Error: 2:3 expected semicolon or line break
// Error: 1:4-1:5 cannot join integer with integer
{1 2}

// Should output `2`.
// Error: 2:12 expected semicolon or line break
// Error: 1:22 expected semicolon or line break
{let x = -1 let y = 3 x + y}

// Should output `3`.
{
    // Error: 9-12 expected identifier, found string
    for "v"

    // Error: 10 expected keyword `in`
    for v let z = 1 + 2

    z
}

---
// Ref: false
// Error: 2:1 expected closing brace
{

---
// Ref: false
// Error: 1-2 unexpected closing brace
}
