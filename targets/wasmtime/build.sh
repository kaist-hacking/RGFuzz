#!/bin/bash

cd wasmtime-wrapper
cargo build --release
cargo build --target aarch64-unknown-linux-gnu --release
cargo build --target riscv64gc-unknown-linux-gnu --release
cargo build --target s390x-unknown-linux-gnu --release
cd ..