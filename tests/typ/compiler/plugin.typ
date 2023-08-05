// Test loading wasm plugins and using them.
// Ref: false

---
#let p = plugin("/files/hello.wasm")
#test(p.hello(), bytes("Hello from wasm!!!"))
#test(p.double_it(bytes("hey!")), bytes("hey!.hey!"))
#test(p.shuffle(bytes("value1"),bytes("value2"),bytes("value3")), bytes("value3-value1-value2"))

---
#let p = plugin("/files/hello.wasm")
// Error: 1:15-1:24 unexpected argument
#test(p.hello(bytes("")), bytes("Hello from wasm!!!"))

---
#let p = plugin("/files/hello.wasm")
// Error: 1:20-1:22 plugin errored with: 'This is an `Err`'
#test(p.returns_err(),"")

---
#let p = plugin("/files/hello.wasm")
// Error: 1:19-1:21 plugin panicked
#test(p.will_panic(),"")
