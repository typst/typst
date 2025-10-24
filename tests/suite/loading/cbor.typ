--- cbor-decode-deprecated render ---
// Warning: 15-21 `cbor.decode` is deprecated, directly pass bytes to `cbor` instead
// Hint: 15-21 it will be removed in Typst 0.15.0
#let _ = cbor.decode

--- cbor-decode-integer render ---
#import "edge-case.typ": cbor-integers
#let data = cbor(cbor-integers)

#for (name, result) in data.representable {
  assert.eq(
    type(result),
    int,
    message: "failed to decode " + name,
  )
}

#for (name, result) in data.large {
  assert.eq(
    type(result),
    float,
    message: "failed to approximately decode " + name,
  )
}

--- cbor-encode-bytes render ---
#let value = bytes("Typst")
#test(cbor(cbor.encode(value)), value)

--- cbor-encode-any render ---
#import "edge-case.typ": special-types
#for value in special-types {
  test(
    cbor.encode(value),
    cbor.encode(repr(value)),
  )
}
