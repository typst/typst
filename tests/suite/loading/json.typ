--- json paged ---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

--- json-with-bom paged ---
// Error: 7-43 failed to parse JSON (unexpected Byte Order Mark at 1:1)
// Hint: 7-43 JSON requires UTF-8 without a BOM
#json(bytes("\u{FEFF}{\"name\": \"BOM\"}"))

--- json-invalid paged ---
// Error: "/assets/data/bad.json" 3:14 failed to parse JSON (expected value at line 3 column 14)
#json("/assets/data/bad.json")

--- json-decode-deprecated paged ---
// Warning: 15-21 `json.decode` is deprecated, directly pass bytes to `json` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = json.decode

--- issue-3363-json-large-number paged ---
// Big numbers (larger than what i64 can store) should just lose some precision
// but not overflow
#let bignum = json("/assets/data/big-number.json")
#bignum

--- json-decode-number paged ---
#import "edge-case.typ": large-integer, representable-integer

#for (name, source) in representable-integer {
  assert.eq(
    type(json(bytes(source))),
    int,
    message: "failed to decode " + name,
  )
}

#for (name, source) in large-integer {
  assert.eq(
    type(json(bytes(source))),
    float,
    message: "failed to approximately decode " + name,
  )
}

--- json-encode-any paged ---
#import "edge-case.typ": special-types-for-human
#for value in special-types-for-human {
  test(
    json.encode(value),
    json.encode(repr(value)),
  )
}
