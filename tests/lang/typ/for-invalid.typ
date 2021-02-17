// Test invalid for loop syntax.

---
// Error: 5-5 expected identifier
#for

// Error: 7-7 expected keyword `#in`
#for v

// Error: 10-10 expected expression
#for v in

// Error: 15-15 expected body
#for v in iter

---
// Should output `v in iter`.
// Error: 5 expected identifier
#for
v in iter {}

// Should output `A thing`.
// Error: 7-10 expected identifier, found string
A#for "v" thing.

// Should output `in iter`.
// Error: 6-9 expected identifier, found string
#for "v" in iter {}

// Should output `+ b in iter`.
// Error: 7 expected keyword `#in`
#for a + b in iter {}
