--- toml ---
// Test reading TOML data.
#let data = toml("/assets/data/toml-types.toml")
#test(data.string, "wonderful")
#test(data.integer, 42)
#test(data.float, 3.14)
#test(data.boolean, true)
#test(data.array, (1, "string", 3.0, false))
#test(data.inline_table, ("first": "amazing", "second": "greater") )
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

--- toml-encode-any ---
// Anything can be encoded.
// Unsupported types fall back to strings via `repr`, but the specific output can be changed.
#let check(value) = test(
  toml.encode((key: value)),
  toml.encode((key: repr(value))),
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

--- toml-encode-non-table ---
// Error: 14-15 expected dictionary, found integer
#toml.encode(3)

--- toml-decode-non-table ---
// Error: 7-17 failed to parse TOML (expected `.`, `=` at 1:2)
#toml(bytes("3"))
