#!/bin/bash

export UID="$(id -u)"
export GID="$(id -g)"

cd ../../docker/

../eval/cov/wasmtime-all-singlecore.sh
../eval/cov/wasmer-all-singlecore.sh
../eval/cov/wasmedge-all-singlecore.sh
../eval/cov/v8-all-singlecore.sh
../eval/cov/jsc-all-singlecore.sh

cd ../eval/cov