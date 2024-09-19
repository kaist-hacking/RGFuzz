#!/usr/bin/env sh

racket wasmlike.rkt > derp.wat && wat-desugar derp.wat
