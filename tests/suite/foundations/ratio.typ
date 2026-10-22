--- ratio-relative-to eval ---
// Test the `relative-to` function.
#test(50%.relative-to(250%), 125%)
#test(50%.relative-to(100pt), 50pt)
#test(50%.relative-to(25% + 100pt), 12.5% + 50pt)
#test(50%.relative-to(180deg), 90deg)
#test(50%.relative-to(10fr), 5fr)
#test(100%.relative-to(1), 1.0)
#test(50%.relative-to(10.0), 5.0)

--- double-percent-embedded eval ---
// Test for two percent signs in a row.
// Error: 2-7 invalid number suffix: `%%`
#3.1%%

--- double-percent-parens eval ---
// Error: 3-8 invalid number suffix: `%%`
#(3.1%%)
