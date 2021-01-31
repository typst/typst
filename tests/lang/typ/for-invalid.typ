// Test invalid for loop syntax.

---
// Error: 5-5 expected identifier
#for

// Error: 7-7 expected keyword `#in`
#for v

// Error: 11-11 expected expression
#for v #in

// Error: 16-16 expected body
#for v #in iter

---
// Should output `v iter`.
// Error: 2:5-2:5 expected identifier
// Error: 2:3-2:6 unexpected keyword `#in`
#for
v #in iter {}

// Should output `A thing`.
// Error: 7-10 expected identifier, found string
A#for "v" thing.

// Should output `iter`.
// Error: 2:6-2:9 expected identifier, found string
// Error: 1:10-1:13 unexpected keyword `#in`
#for "v" #in iter {}

// Should output `+ b iter`.
// Error: 2:7-2:7 expected keyword `#in`
// Error: 1:12-1:15 unexpected keyword `#in`
#for a + b #in iter {}
