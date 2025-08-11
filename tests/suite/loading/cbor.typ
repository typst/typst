--- cbor-decode-deprecated ---
// Warning: 15-21 `cbor.decode` is deprecated, directly pass bytes to `cbor` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = cbor.decode

--- cbor-encode-bytes ---
#let value = bytes("Typst")
#test(cbor(cbor.encode(value)), value)

--- cbor-encode-any ---
// Anything can be encoded.
// Unsupported types fall back to strings via `repr`, but the specific output can be changed.
#let check(value) = test(
  cbor.encode(value),
  cbor.encode(repr(value)),
)

#check(decimal("2.99792458"))
#check(3.14159pt)
#check(2.99em)
#check(<sec:intro>)
#check(panic)
#check(x => x + 1)
#check(regex("\p{Letter}"))
#check(stroke(red + 5pt))
#check(auto)
