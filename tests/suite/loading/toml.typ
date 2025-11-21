--- toml ---
// Test reading TOML data.
#let data = toml("/assets/data/toml-types.toml")
#test(data.string, "wonderful")
#test(data.integer, 42)
#test(data.float, 3.14)
#test(data.boolean, true)
#test(data.array, (1, "string", 3.0, false))
#test(data.inline_table, ("first": "amazing", "second": "greater"))
#test(data.table.element, 5)
#test(data.table.others, (false, "indeed", 7))
#test(data.date_time, datetime(
  year: 2023,
  month: 2,
  day: 1,
  hour: 15,
  minute: 38,
  second: 57,
))
#test(data.date_time2, datetime(
  year: 2023,
  month: 2,
  day: 1,
  hour: 15,
  minute: 38,
  second: 57,
))
#test(data.date, datetime(
  year: 2023,
  month: 2,
  day: 1,
))
#test(data.time, datetime(
  hour: 15,
  minute: 38,
  second: 57,
))

--- toml-invalid ---
// Error: "/assets/data/bad.toml" 1:16-2:1 failed to parse TOML (expected `.`, `=`)
#toml("/assets/data/bad.toml")

--- toml-decode-deprecated ---
// Warning: 15-21 `toml.decode` is deprecated, directly pass bytes to `toml` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = toml.decode

--- toml-decode-integer ---
#import "edge-case.typ": representable-integer

#for (name, source) in representable-integer {
  assert.eq(
    // The `key` trick is necessary because a TOML documents must be a table.
    type(toml(bytes("key = " + source)).key),
    int,
    message: "failed to decode " + name,
  )
}

--- toml-decode-integer-too-large ---
// If an integer cannot be represented losslessly, an error must be thrown.
// https://toml.io/en/v1.0.0#integer

#import "edge-case.typ": large-integer
// Error: 7-55 failed to parse TOML (number too large to fit in target type at 1:7)
#toml(bytes("key = " + large-integer.i64-max-plus-one))

--- toml-decode-integer-too-small ---
#import "edge-case.typ": large-integer
// Error: 7-56 failed to parse TOML (number too small to fit in target type at 1:7)
#toml(bytes("key = " + large-integer.i64-min-minus-one))

--- toml-encode-any ---
#import "edge-case.typ": special-types-for-human
#for value in special-types-for-human {
  test(
    toml.encode((key: value)),
    toml.encode((key: repr(value))),
  )
}

--- toml-encode-non-table ---
// Error: 14-15 expected dictionary, found integer
#toml.encode(3)

--- toml-decode-non-table ---
// Error: 7-17 failed to parse TOML (expected `.`, `=` at 1:2)
#toml(bytes("3"))
