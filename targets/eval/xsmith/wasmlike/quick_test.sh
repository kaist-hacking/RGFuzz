#!/usr/bin/env sh

echo "Output:"
echo "----------------"
racket wasmlike.rkt --debug-show-s-exp-tree true --max-depth 6 --function-definition-falloff 4 > derp.wat && wat-desugar derp.wat

rm -f derp.wasm
wat2wasm derp.wat

echo "Result:"
echo "----------------"
node ../node/load-node.js derp.wasm
