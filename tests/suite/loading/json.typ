--- json eval ---
// Test reading JSON data.
#let data = json("/assets/data/zoo.json")
#test(data.len(), 3)
#test(data.at(0).name, "Debby")
#test(data.at(2).weight, 150)

// Test reading through path type.
#let data-from-path = json(path("/assets/data/zoo.json"))
#test(data-from-path, data)

--- json-with-bom eval ---
// Error: 7-43 failed to parse JSON (unexpected Byte Order Mark at 1:1)
// Hint: 7-43 JSON requires UTF-8 without a BOM
#json(bytes("\u{FEFF}{\"name\": \"BOM\"}"))

--- json-invalid eval ---
// Error: "/assets/data/bad.json" 3:14 failed to parse JSON (expected value at line 3 column 14)
#json("/assets/data/bad.json")

--- issue-3363-json-large-number paged ---
// Big numbers (larger than what i64 can store) should just lose some precision
// but not overflow
#let bignum = json("/assets/data/big-number.json")
#bignum

--- json-decode-number eval ---
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

--- json-encode-any eval ---
#import "edge-case.typ": special-types-for-human
#for value in special-types-for-human {
  test(
    json.encode(value),
    json.encode(repr(value)),
  )
}

--- json-lines-values eval ---
#let source = bytes(
  "{\"name\":\"猫\"}\n[1,2]\n\"a\\nb\"\n42\n1.5\ntrue\nfalse\nnull",
)
#test(
  json(source, lines: true),
  ((name: "猫"), (1, 2), "a\nb", 42, 1.5, true, false, none),
)

--- json-lines-endings eval ---
#for ending in ("\n", "\r\n") {
  for trailing in ("", ending) {
    let source = bytes(" 1 " + ending + "\t2\t" + trailing)
    test(json(source, lines: true), (1, 2))
  }
}

--- json-lines-empty eval ---
#test(json(bytes(""), lines: true), ())

--- json-lines-single-value eval ---
#test(json(bytes("[1,2]"), lines: true), ((1, 2),))
#test(json(bytes("[1,2]"), lines: false), (1, 2))
#test(json(bytes("[1,2]")), (1, 2))

--- json-lines-file eval ---
#let expected = ((name: "猫"), none, (1, 2))
#test(json("jsonl/valid.jsonl", lines: true), expected)
#test(json(path("jsonl/valid.jsonl"), lines: true), expected)

--- json-lines-invalid-file eval ---
// Error: "tests/suite/loading/jsonl/invalid.jsonl" 3:10 failed to parse JSON Lines (expected value)
#json("jsonl/invalid.jsonl", lines: true)

--- json-lines-blank-line eval ---
#let source = bytes("null\n\ntrue")
// Error: 7-13 failed to parse JSON Lines (EOF while parsing a value at 2:1)
#json(source, lines: true)

--- json-lines-trailing-blank-line eval ---
#let source = bytes("null\n\n")
// Error: 7-13 failed to parse JSON Lines (EOF while parsing a value at 2:1)
#json(source, lines: true)

--- json-lines-whitespace-line eval ---
#let source = bytes("null\n \t\n")
// Error: 7-13 failed to parse JSON Lines (EOF while parsing a value at 2:2)
#json(source, lines: true)

--- json-lines-multiple-values eval ---
#let source = bytes("null\n1 2\n")
// Error: 7-13 failed to parse JSON Lines (trailing characters at 2:3)
#json(source, lines: true)

--- json-lines-multiline-value eval ---
#let source = bytes("null\n{\n\"a\":1\n}")
// Error: 7-13 failed to parse JSON Lines (EOF while parsing an object at 2:1)
#json(source, lines: true)

--- json-lines-with-bom eval ---
#let source = bytes("\u{FEFF}null\n")
// Error: 7-13 failed to parse JSON (unexpected Byte Order Mark at 1:1)
// Hint: 7-13 JSON requires UTF-8 without a BOM
#json(source, lines: true)

--- json-lines-invalid-utf8 eval ---
#let source = bytes((110, 117, 108, 108, 10, 34, 255, 34))
// Error: 7-13 failed to parse JSON Lines (invalid unicode code point at 2:3)
#json(source, lines: true)

--- json-lines-disabled eval ---
#let source = bytes("null\ntrue")
// Error: 7-13 failed to parse JSON (trailing characters at line 2 column 1 at 2:1)
#json(source)
