--- relative-fields paged ---
// Test relative length fields.
#test((100% + 2em + 2pt).ratio, 100%)
#test((100% + 2em + 2pt).length, 2em + 2pt)
#test((100% + 2pt).length, 2pt)
#test((100% + 2pt - 2pt).length, 0pt)
#test((56% + 2pt - 56%).ratio, 0%)

--- double-percent-embedded paged ---
// Test for two percent signs in a row.
// Error: 2-7 invalid number suffix: %%
#3.1%%

--- double-percent-parens paged ---
// Error: 3-8 invalid number suffix: %%
#(3.1%%)
