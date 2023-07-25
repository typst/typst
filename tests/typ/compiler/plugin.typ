// Test loading wasm plugins and using them.
// Ref: false

---
#let p = plugin("/files/hello.wasm")
#test(p.hello(), "Hello from wasm!!!")
#test(p.double_it("hey!"), "hey!.hey!")
#test(p.shuffle("value1","value2","value3"), "value3-value1-value2")

---
#let p = plugin("/files/hello.wasm")
// Error: 1:15-1:17 unexpected argument
#test(p.hello(""), "Hello from wasm!!!")

---
#let p = plugin("/files/hello.wasm")
// Error: 1:20-1:22 plugin errored with: "This is an `Err`" with code: 1
#test(p.returns_err(),"")

---
#let p = plugin("/files/hello.wasm")
// Error: 1:19-1:21 plugin panicked
#test(p.will_panic(),"")
