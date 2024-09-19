#!/bin/bash

# Generates a WebAssembly program using Wasmlike. Runs it through the predicate to check if it ever computes a NaN. Returns 1 if it does; returns 0 if not.

# 1st parameter is the seed

# where the wasm file is generated
dir=../wrong-code-5

options="--with-safe-memory-loads true --max-depth 9 --function-definition-falloff 4 --with-floating-point true --with-loop-parameters false"

racket /local/work/webassembly-sandbox/wasmlike/wasmlike.rkt $options --seed $1 > $dir/original_program.wat
wat2wasm $dir/original_program.wat -o $dir/original_program.wasm
../predicate.py $dir/original_program.wasm
