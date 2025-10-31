--- panic paged diagnostic ---
// Test panic.
// Error: 2-9 panicked
#panic()

--- panic-with-int paged diagnostic ---
// Test panic.
// Error: 2-12 panicked with: 123
#panic(123)

--- panic-with-str paged diagnostic ---
// Test panic.
// Error: 2-24 panicked with: "this is wrong"
#panic("this is wrong")
