// Test Out Of Bound read/write in WebAssembly plugins communication.
// Ref: false

---
#let p = plugin("/files/plugin-oob.wasm")

// Error: 2-14 plugin tried to read out of bounds: pointer 0x40000000 is out of bounds for read of length 1
#p.read_oob()

---
#let p = plugin("/files/plugin-oob.wasm")

// Error: 2-27 plugin tried to write out of bounds: pointer 0x40000000 is out of bounds for write of length 3
#p.write_oob(bytes("xyz"))
