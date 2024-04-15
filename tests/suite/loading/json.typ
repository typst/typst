--- json ---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

--- json-invalid ---
// Error: 7-30 failed to parse JSON (expected value at line 3 column 14)
#json("/assets/data/bad.json")

--- issue-3363-json-large-number ---
// Big numbers (larger than what i64 can store) should just lose some precision
// but not overflow
#let bignum = json("/assets/data/big-number.json")
#bignum
