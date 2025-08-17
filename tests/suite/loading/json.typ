--- json ---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

--- json-not-found ---
// Error: 7-18 file not found (searched at tests/suite/loading/nope.json)
#json("nope.json")

--- json-invalid ---
// Error: "/assets/data/bad.json" 3:14 failed to parse JSON (expected value at line 3 column 14)
#json("/assets/data/bad.json")

--- json-decode-deprecated ---
// Warning: 15-21 `json.decode` is deprecated, directly pass bytes to `json` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = json.decode

--- issue-3363-json-large-number ---
// Big numbers (larger than what i64 can store) should just lose some precision
// but not overflow
#let bignum = json("/assets/data/big-number.json")
#bignum

--- json-default ---
// Use the default balue
#let data_def = json("nope_default.json", default: none)
#test(data_def, none)

