// Test that errors in show rules are delayed: There can be multiple at once.

---
// Error: 21-34 panicked with: "hey1"
#show heading: _ => panic("hey1")

// Error: 20-33 panicked with: "hey2"
#show strong: _ => panic("hey2")

= Hello
*strong*
