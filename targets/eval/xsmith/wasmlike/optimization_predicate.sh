#!/usr/bin/env zsh

# Exit the script if any subcommand fails.

# The Wasm file is given as the first and only argument to the script.
WASM=$1

# Run the Wasm in Wasmtime and `grep` for our target bug's panic
# message.
#../wasmer/load-wasmer/target/debug/load-wasmer --compiler llvm --engine universal --optimize $WASM 2>&1 | grep 'segmentation fault' #--quiet 'segmentation fault'
OPTIMIZED=`../wasmer/load-wasmer/target/debug/load-wasmer --compiler llvm --engine universal --optimize $WASM`


if [[ $? == 139 ]]
then
  UNOPTIMIZED=`../wasmer/load-wasmer/target/debug/load-wasmer --compiler llvm --engine universal $WASM`
  if [[ $UNOPTIMIZED =~ "[0-9a-f]{8}" ]]
  then
    exit 0
  else
    exit 1
  fi
else
  exit 1
fi
