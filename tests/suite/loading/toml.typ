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
// Error: "/assets/data/bad.toml" #15-#16 failed to parse TOML (expected `.`, `=` at line 1 column 16)
#toml("/assets/data/bad.toml")
