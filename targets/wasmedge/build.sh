#!/bin/bash

cd wasmedge-wrapper
cargo build --release
cargo build --target aarch64-unknown-linux-gnu --release
cd ..