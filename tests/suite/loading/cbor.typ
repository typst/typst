--- cbor-decode-integer eval ---
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

--- cbor-encode-bytes eval ---
#let value = bytes("Typst")
#test(cbor(cbor.encode(value)), value)

--- cbor-encode-any eval ---
#import "edge-case.typ": special-types
#for value in special-types {
  test(
    cbor.encode(value),
    cbor.encode(repr(value)),
  )
}
