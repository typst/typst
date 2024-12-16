--- relative-fields ---
// Test relative length fields.
#test((100% + 2em + 2pt).ratio, 100%)
#test((100% + 2em + 2pt).length, 2em + 2pt)
#test((100% + 2pt).length, 2pt)
#test((100% + 2pt - 2pt).length, 0pt)
#test((56% + 2pt - 56%).ratio, 0%)

--- double-percent ---
// Test for two percent signs in a row.
#3.1%%

--- double-percent-error ---
// Error: 7-8 the character `%` is not valid in code
#(3.1%%)
