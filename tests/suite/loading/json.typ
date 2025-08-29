--- json ---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

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

--- json-encode-any ---
// Anything can be encoded.
// Unsupported types fall back to strings via `repr`, but the specific output can be changed.
#let check(value) = test(
  json.encode(value),
  json.encode(repr(value)),
)

#check(bytes("Typst"))
#check(decimal("2.99792458"))
#check(3.14159pt)
#check(2.99em)
#check(<sec:intro>)
#check(panic)
#check(x => x + 1)
#check(regex("\p{Letter}"))
#check(stroke(red + 5pt))
#check(auto)
