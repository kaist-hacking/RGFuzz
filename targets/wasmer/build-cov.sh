#!/bin/bash

source ~/.profile

cd wasmer-wrapper
LLVM_SYS_150_PREFIX=$(pwd)/../llvm-15/build RUSTFLAGS="-C instrument-coverage" cargo build --release
# RUSTFLAGS="-C instrument-coverage" cargo build --target aarch64-unknown-linux-gnu --release
# RUSTFLAGS="-C instrument-coverage" cargo build --target riscv64gc-unknown-linux-gnu --release
cd ..