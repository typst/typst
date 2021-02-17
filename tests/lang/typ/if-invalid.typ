// Test invalid if syntax.

---
// Error: 4 expected expression
#if

// Error: 4 expected expression
{if}

// Error: 6 expected body
#if x

// Error: 1-6 unexpected keyword `else`
#else {}

---
// Should output `x`.
// Error: 4 expected expression
#if
x {}

// Should output `something`.
// Error: 6 expected body
#if x something

// Should output `A thing.`
// Error: 20 expected body
A#if false {} #else thing
