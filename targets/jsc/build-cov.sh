#!/bin/bash

cd WebKit
export WEBKIT_OUTPUTDIR="$(pwd)/../builds/x64"
Tools/Scripts/build-webkit --jsc-only --release --cmakeargs='-DCMAKE_C_COMPILER=clang-19 -DCMAKE_CXX_COMPILER=clang++-19 -DCMAKE_CXX_FLAGS="-Wno-constant-conversion -Wno-deprecated-declarations -fprofile-instr-generate -fcoverage-mapping"'
cd ..
