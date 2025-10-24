// Test WebAssembly plugins.

--- plugin-basic paged ---
#let p = plugin("/assets/plugins/hello.wasm")
#test(p.hello(), bytes("Hello from wasm!!!"))
#test(p.double_it(bytes("hey!")), bytes("hey!.hey!"))
#test(
  p.shuffle(bytes("value1"), bytes("value2"), bytes("value3")),
  bytes("value3-value1-value2"),
)

--- plugin-func paged ---
#let p = plugin("/assets/plugins/hello.wasm")
#test(type(p.hello), function)
#test(("a", "b").map(bytes).map(p.double_it), ("a.a", "b.b").map(bytes))

--- plugin-import paged ---
#import plugin("/assets/plugins/hello.wasm"): hello, double_it

#test(hello(), bytes("Hello from wasm!!!"))
#test(double_it(bytes("hey!")), bytes("hey!.hey!"))

--- plugin-transition paged ---
#let empty = plugin("/assets/plugins/hello-mut.wasm")
#test(str(empty.get()), "[]")

#let hello = plugin.transition(empty.add, bytes("hello"))
#test(str(empty.get()), "[]")
#test(str(hello.get()), "[hello]")

#let world = plugin.transition(empty.add, bytes("world"))
#let hello_you = plugin.transition(hello.add, bytes("you"))

#test(str(empty.get()), "[]")
#test(str(hello.get()), "[hello]")
#test(str(world.get()), "[world]")
#test(str(hello_you.get()), "[hello, you]")

#let hello2 = plugin.transition(empty.add, bytes("hello"))
#test(hello == world, false)
#test(hello == hello2, true)

--- plugin-wrong-number-of-arguments paged ---
#let p = plugin("/assets/plugins/hello.wasm")

// Error: 2-20 plugin function takes 0 arguments, but 1 was given
#p.hello(bytes(""))

--- plugin-wrong-argument-type paged ---
#let p = plugin("/assets/plugins/hello.wasm")

// Error: 10-14 expected bytes, found boolean
// Error: 27-29 expected bytes, found integer
#p.hello(true, bytes(()), 10)

--- plugin-error paged ---
#let p = plugin("/assets/plugins/hello.wasm")

// Error: 2-17 plugin errored with: This is an `Err`
#p.returns_err()

--- plugin-panic paged ---
#let p = plugin("/assets/plugins/hello.wasm")

// Error: 2-16 plugin panicked: wasm `unreachable` instruction executed
#p.will_panic()

--- plugin-out-of-bounds-read paged ---
#let p = plugin("/assets/plugins/plugin-oob.wasm")

// Error: 2-14 plugin tried to read out of bounds: pointer 0x40000000 is out of bounds for read of length 1
#p.read_oob()

--- plugin-out-of-bounds-write paged ---
#let p = plugin("/assets/plugins/plugin-oob.wasm")

// Error: 2-27 plugin tried to write out of bounds: pointer 0x40000000 is out of bounds for write of length 3
#p.write_oob(bytes("xyz"))
