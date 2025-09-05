// SKIP
// Edge cases in serialization and deserialization.
// They can be imported when testing decoding functions.

#let representable-integer = (
  i64-max: "9223372036854775807",
  i64-min: "-9223372036854775808",
)

#let large-integer = (
  i64-max-plus-one: "9223372036854775808",
  i64-min-minus-one: "-9223372036854775809",
  u64-max: "18446744073709551615",
  u64-max-plus-one: "18446744073709551616",
  i128-max: "170141183460469231731687303715884105727",
  i128-min: "-170141183460469231731687303715884105728",
  u128-max: "340282366920938463463374607431768211455",
)

// The following is generated from the rust script in PR 6836.
#let cbor-integers = bytes(
  (162, 109, 114, 101, 112, 114, 101, 115, 101, 110, 116, 97, 98, 108, 101, 162, 103, 105, 54, 52, 95, 109, 97, 120, 27, 127, 255, 255, 255, 255, 255, 255, 255, 103, 105, 54, 52, 95, 109, 105, 110, 59, 127, 255, 255, 255, 255, 255, 255, 255, 101, 108, 97, 114, 103, 101, 167, 112, 105, 54, 52, 95, 109, 97, 120, 95, 112, 108, 117, 115, 95, 111, 110, 101, 27, 128, 0, 0, 0, 0, 0, 0, 0, 113, 105, 54, 52, 95, 109, 105, 110, 95, 109, 105, 110, 117, 115, 95, 111, 110, 101, 59, 128, 0, 0, 0, 0, 0, 0, 0, 103, 117, 54, 52, 95, 109, 97, 120, 27, 255, 255, 255, 255, 255, 255, 255, 255, 112, 117, 54, 52, 95, 109, 97, 120, 95, 112, 108, 117, 115, 95, 111, 110, 101, 194, 73, 1, 0, 0, 0, 0, 0, 0, 0, 0, 104, 105, 49, 50, 56, 95, 109, 97, 120, 194, 80, 
127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 104, 105, 49, 50, 56, 95, 109, 105, 110, 195, 80, 127, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 104, 117, 49, 50, 56, 95, 109, 97, 120, 194, 80, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255),
)

// These special Typst types are supported by neither human-readable nor binary
// data formats. They should fall back to strings via `repr` when encoded, but
// the specific output can be changed.
#let special-types = (
  decimal("2.99792458"),
  3.14159pt,
  2.99em,
  <sec:intro>,
  panic,
  x => x + 1,
  regex("\p{Letter}"),
  stroke(red + 5pt),
  auto,
)

// These special Typst types are not supported by human-readable data formats.
#let special-types-for-human = special-types + (bytes("Typst"),)
