--- relative-fields eval ---
// Test relative length fields.
#test((100% + 2em + 2pt).ratio, 100%)
#test((100% + 2em + 2pt).length, 2em + 2pt)
#test((100% + 2pt).length, 2pt)
#test((100% + 2pt - 2pt).length, 0pt)
#test((56% + 2pt - 56%).ratio, 0%)

--- relative-relative-to eval ---
// Test the `relative-to` function.
#test((100% + 0pt).relative-to(10pt), 10pt)
#test((50% + 3pt).relative-to(100pt), 53pt)
#test((50% + 3pt).relative-to(20% + 100pt), 10% + 53pt)

--- double-percent-embedded eval ---
// Test for two percent signs in a row.
// Error: 2-7 invalid number suffix: `%%`
#3.1%%

--- double-percent-parens eval ---
// Error: 3-8 invalid number suffix: `%%`
#(3.1%%)
