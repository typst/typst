---
description: |
  You need to use external code for performance 
  reason or to avoid reinventing the wheel? This guide
  explains how to integrate your code as a plugin.
---

# Guide for plugin development (Advanced users)

This page helps you get started building plugins for typst. This is an advanced functionality and beginners might have diffculties following this guide.

Typst is capable of interfacing with plugins compiled to WebAssembly via its `plugin` function.

Once your program has been compiled to WebAssembly, it also needs to respect [the protocol](#protocol). 

Typst will run this code in isolation from your system, which means printing and reading files will not be supported for security reasons.

Many compilers will use the [wasi ABI](https://wasi.dev/) by default or as their only option (e.g. emscripten), which allows printing, reading file etc. This will not work with typst, you will need to either compile to a different target if possible or stub your library using a tool (see below).

Typst will be able to run your code compiled targeting wasi if all the functions is the wasi protocol are stubbed (replaced by dummy implementions). Which you can easily achieve with [this tool](https://github.com/astrale-sharp/wasm-minimal-protocol/blob/master/wasi-stub/README.md#wasi-stub) that you can find on the repository below.


You should check out this [link](https://github.com/astrale-sharp/wasm-minimal-protocol). The repo contains:
- A list of examples of plugin implementations.
- A test runner for these examples.
- Wrappers to help you write your plugin in Rust. (Zig wrappers in development)
- A stubber for wasi [here](https://github.com/astrale-sharp/wasm-minimal-protocol#you-need-to-stub-a-webassembly-plugin)


# Protocol

This section describes the protocol Typst expects plugins to implement. This protocol sends and receive byte slices with Typst as the host.

Types and functions are described using WAT syntax.

## Compilation

This protocol is only meant to be used by plugins compiled to 32-bits WebAssembly.

A plugin should compile to a shared WebAssembly library.

## Imports

Valid plugins need to import two functions (that will be provided by the runtime):

- `(import "typst_env" "wasm_minimal_protocol_write_args_to_buffer" (func (param i32)))`

  The argument is a pointer to a buffer (`ptr`).

  Write the arguments for the current function into the buffer pointed at by `ptr`.

  Each function for the protocol receives lengths as its arguments (see [User-defined functions](#user-defined-functions)). The capacity of the buffer pointed at by `ptr` should be at least the sum of all those lengths.

- `(import "typst_env" "wasm_minimal_protocol_send_result_to_host" (func (param i32 i32)))`

  The first parameter is a pointer to a buffer (`ptr`), the second is the length of the buffer (`len`).

  Send `len` and `ptr` to host memory. The buffer must not be freed by the end of the function: it will be freed by the runtime by calling [`wasm_minimal_protocol_send_result_to_host`](#exports).

  If the message should be interpreted as an error message (see [User-defined functions](#user-defined-functions)), it should be encoded as UTF-8.

### Note

If [`wasm_minimal_protocol_send_result_to_host`](#exports) calls `free` (or a similar routine), be careful that the buffer does not point to static memory.

## Exports

Valid plugins need to export a function named `wasm_minimal_protocol_send_result_to_host`, that has signature `func (param i32 i32)`.

This function will be used by the runtime to free the block of memory returned by a [user-defined](#user-defined-functions) function.

## User-defined functions

To conform to the protocol, an exported function should:

- Take `n` arguments `a₁`, `a₂`, ..., `aₙ` of type `u32` (interpreted as lengths, so `usize/size_t` may be preferable), and return one `i32`. We will call the return `return_code`.
- The function should first allocate a buffer `buf` of length `a₁ + a₂ + ⋯ + aₙ`, and call `wasm_minimal_protocol_write_args_to_buffer(buf.ptr)`.
- The `a₁` first bytes of the buffer constitute the first argument, the `a₂` next bytes the second argument, and so on.
- Before returning, the function should call `wasm_minimal_protocol_send_result_to_host` to send its result back to the host.
- To signal success, `return_code` must be `0`.
- To signal an error, `return_code` must be `1`. The sent buffer is then interpreted as an error message, and must be encoded as UTF-8.
