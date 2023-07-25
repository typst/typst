---
description: |
  You need to use external code for performance 
  reason or to avoid re inventing the wheel? This guide
  explains how to integrate your code as a plugin.
---

# Guide for plugin development

This page helps you get started building plugins for typst.

External code can be used from typst once it is compiled to WebAssembly. 

Once your program has been compiled to WebAssembly, it also need to respect [this](../dev/plugins.md) protocol. 

Typst will run this code in isolation from your system which means printing and reading file will not be supported for security reasons.

Many compilers will use the [wasi ABI](https://wasi.dev/) by default or as their only option (e.g. emscripten), which allows printing, reading file etc. This will not work with typst, you will need to either compile to a different target if possible or stub your library using a tool (see below).

Typst will be able to run your code compiled targeting wasi if all the functions is the wasi protocol are stubbed (replaced by dummy implementions). Which you can easily achieve with [this tool](https://github.com/astrale-sharp/wasm-minimal-protocol/blob/master/wasi-stub/README.md#wasi-stub) that you can find on the repository below.


You should check out this [link](https://github.com/astrale-sharp/wasm-minimal-protocol). The repo contains:
- A list of examples of plugins implementation.
- A test runner for these examples
- Wrappers to help you write your plugin in Rust. (Zig wrappers in development)
- A stubber for wasi ()