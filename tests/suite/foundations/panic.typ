// Test the panic function.

--- panic-empty eval ---
// Test panic with no args.
// Error: 2-9 panicked
#panic()

--- panic-int eval ---
// We can panic with a random value.
// Error: 2-12 panicked with: 123
#panic(123)

--- panic-str eval ---
// But usually we panic with an error message.
// Error: 2-24 panicked with: this is wrong
#panic("this is wrong")

--- panic-multiline eval ---
// Test panic with a multiline string.
// Error: 1:2-2:7 panicked with: oops\noops\noops
#panic("oops\noops
oops")

--- panic-raw-newline eval ---
// Test panic with raw text containing escaped and normal newlines.
// Error: 1:2-2:5 panicked with: raw(text: "\\n\n\\n", block: false)
#panic(`\n
\n`)

--- issue-5219-panic-escaped-quotes eval ---
// Error: 2-42 panicked with: use an identifier like "math"
#panic("use an identifier like \"math\"")
