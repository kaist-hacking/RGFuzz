#!/bin/bash

# Runs Binaryen's WebAssembly test case reducer

# 1st and 2nd params are targets from wasm-all-config surrounded by quotes e.g. "NodeJS"
# 3rd param is the path to the orignial test case

tmux new-session -d -s "wasm-reduce-session" \
	wasm-reduce $3 "--command=./compare.py -c /local/work/harness/configs/wasm-all-config.yml -a $1 -b $2 -p test.wasm --compare" -t test.wasm -w work.wasm

tmux attach -t wasm-reduce-session
