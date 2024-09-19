# WASM
[![Build](https://github.com/misalcedo/wasm-ast/actions/workflows/build.yml/badge.svg)](https://github.com/misalcedo/wasm-ast/actions/workflows/build.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-yellowgreen.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io Version](https://img.shields.io/crates/v/wasm-ast.svg)](https://crates.io/crates/wasm-ast)
[![Docs.rs Version](https://docs.rs/wasm-ast/badge.svg)](https://docs.rs/wasm-ast)

A Rust-native WebAssembly syntax model useful for generating, parsing, and emitting WebAssembly code.

## Design
WASM-AST is designed with minimal validation. The goal is to closely model the WASM syntax specification in order to allow valid and invalid abstract syntax trees. Lastly, modules cannot be mutated once built.

## Features
### Parser
A parser for binary WebAssembly format. Attempts to maintain as much of the binary information as possible.

### Text
A parser for the text and binary WebAssembly formats. The text format is transformed to binary, then passed to the binary parser. Some information may be lost in the text to binary conversion.

### Emitter
Emits binary WebAssembly format for a module.


## Usage
To use `wasm-ast`, first add this to your `Cargo.toml`:

```toml
[dependencies]
wasm-ast = "0.1.0"
```

Then, add this to your crate:

```rust
use wasm_ast::model::Module;

fn main() {
    let mut builder = Module::builder();
    let module = builder.build();
}
```

## Examples

Create an empty WASM module:

```rust
use wasm_ast::model::Module;

fn main() {
    let module = Module::empty();
}
```

Additional (i.e., more useful) examples can be found in the repository.

## Stability
The interface is considered stable. No breaking changes will be introduced until the next major version (e.g. `1.0`).

# Issues
Please file any issues for areas where this crate does not properly adhere to the WebAssembly standard.

## License

Licensed under Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://apache.org/licenses/LICENSE-2.0)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be 
licensed as above, without any additional terms or conditions.
