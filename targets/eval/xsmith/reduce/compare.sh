#!/usr/bin/env bash


# !!!
# Kept for historical purposes, and if Binaryen's optimizer/reducer ever
# supports the multivalue proposal.
# The calling convention for the reducer differs between wasm-reduce (binaryen) and 
# wasm-tools shrink (bytecode alliance). Consider using compare.py instead
#
WASM_REDUCE=wasm-reduce
PROGRAM_T="compare-program.wat"
PROGRAM="compare-program.wasm"
XSMITH=/local/work/webassembly-sandbox/wasmlike/wasmlike.rkt
#XSMITH=/Users/GW/Documents/flux/webassembly-sandbox/wasmlike/wasmlike.rkt

POSITIONAL=()
while [[ $# -gt 0 ]]; do
  key="$1"

  case $key in
    -a|--runtime-a)
      RUNTIME_A="$2"
      shift # past argument
      shift # past value
      ;;
    -b|--runtime-b)
      RUNTIME_B="$2"
      shift # past argument
      shift # past value
      ;;
    -x|--xsmith-options)
      XSMITH_OPTIONS="$2"
      XSMITH_OPTIONS_GIVEN=1
      shift # past argument
      shift # past value
      ;;
    -p|--program)
      PROGRAM="$2"
      PROGRAM_GIVEN=1
      shift
      shift
      ;;
    -h|--help)
      echo "Usage: ./compare --runtime-a '.../load-wasmer --compiler cranelift' --runtime-b '.../load-wasmer --compiler llvm'"
      echo "           --xsmith-options --seed 123456"
      echo "Options:"
      echo "-a|--runtime-a        The first runtime. Give the runtime and the options, but not the wasm program. The"
      echo "                        wasm program is inserted as a final positional argument by this script"
      echo "-b|--runtime-b        The second runtime."
      echo "-x|--xsmith-options   The options to pass to xsmith to generate the wasm program. Mututally exclusive with '-p'."
      echo "                        Meant to be used to verify that two outputs are different."
      echo "-p|--program          An already generated program. Mutually exclusive with '-x'."
      echo "                        Meant to be used when coupled with wasm-reduce."
      echo "-h|--help             Display this help"
      exit 0
      ;;
    *)    # unknown option
      echo "Usage: ./compare --runtime-a '.../load-wasmer --compiler cranelift' --runtime-b '.../load-wasmer --compiler llvm'"
      echo "           --xsmith-options '--seed 123456"''
      exit 1  
      ;;
  esac
done

if [[ $XSMITH_OPTIONS_GIVEN && $PROGRAM_GIVEN ]]; then
  echo "ERROR: Must give either xsmith generation options or a program, not both"
  exit 1
fi

if [[ ! ( $XSMITH_OPTIONS_GIVEN || $PROGRAM_GIVEN ) ]]; then
  echo "ERROR: Must give either xsmith generation options or a program"
  exit 1  
fi



# Generate the program
if [[ $XSMITH_OPTIONS_GIVEN ]]; then
  racket $XSMITH $XSMITH_OPTIONS > $PROGRAM_T
  wat2wasm $PROGRAM_T -o $PROGRAM;
fi

# Add the wasm program to the runtime arguments
RUNTIME_A="${RUNTIME_A} ${PROGRAM}"
RUNTIME_B="${RUNTIME_B} ${PROGRAM}"

#echo "RUNTIME A      = ${RUNTIME_A}"
#echo "RUNTIME B      = ${RUNTIME_B}"
#echo "XSMITH OPTIONS = ${XSMITH_OPTIONS}"

# Compare the 2 outputs
if [[ $($RUNTIME_A) = $($RUNTIME_B) ]]; then
  echo "equal"
else
  echo "not equal"
fi

