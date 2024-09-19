#!/bin/bash

source ~/.profile

cd wasmedge-wrapper
WASMEDGE_INCLUDE_DIR=$PWD/../WasmEdge/build/include/api WASMEDGE_LIB_DIR=$PWD/../WasmEdge/build/lib/api cargo build --release
# WASMEDGE_INCLUDE_DIR=$PWD/../WasmEdge/build/include/api WASMEDGE_LIB_DIR=$PWD/../WasmEdge/build/lib/api cargo build --target aarch64-unknown-linux-gnu --release
cd ..