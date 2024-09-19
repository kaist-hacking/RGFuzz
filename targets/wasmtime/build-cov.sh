#!/bin/bash

source ~/.profile

cd wasmtime-wrapper
RUSTFLAGS="-C instrument-coverage" cargo build --release
RUSTFLAGS="-C instrument-coverage" cargo build --target aarch64-unknown-linux-gnu --release
RUSTFLAGS="-C instrument-coverage" cargo build --target riscv64gc-unknown-linux-gnu --release
RUSTFLAGS="-C instrument-coverage" cargo build --target s390x-unknown-linux-gnu --release
cd ..