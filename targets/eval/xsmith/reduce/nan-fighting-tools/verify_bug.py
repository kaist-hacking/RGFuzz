#!/usr/bin/env python3

import sys
import subprocess

a = "\"" + sys.argv[1] + "\""
b = "\"" + sys.argv[2] + "\""
seed = sys.argv[3]

subprocess.run("racket /local/work/webassembly-sandbox/wasmlike/wasmlike.rkt  --max-depth 9 --function-definition-falloff 4 --with-floating-point true --with-loop-parameters false --with-safe-memory-loads true --seed " + seed + " > safe.wat", shell=True)
subprocess.run("racket /local/work/webassembly-sandbox/wasmlike/wasmlike.rkt  --max-depth 9 --function-definition-falloff 4 --with-floating-point true --with-loop-parameters false --with-safe-memory-loads false --seed " + seed + " > unsafe.wat", shell=True)

subprocess.run("wat2wasm safe.wat -o safe.wasm", shell=True)
subprocess.run("wat2wasm unsafe.wat -o unsafe.wasm", shell=True)

subprocess.run("/local/work/webassembly-sandbox/reduce/compare.py -c '/local/work/harness/configs/wasm-all-config.yml' --verify -a " + a +  " -b " + b + " -p safe.wasm", shell=True) 
subprocess.run("/local/work/webassembly-sandbox/reduce/compare.py -c '/local/work/harness/configs/wasm-all-config.yml' --verify -a " + a + " -b " + b + " -p unsafe.wasm", shell=True) 
