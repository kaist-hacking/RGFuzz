#!/bin/bash

cd ..
git submodule init
git submodule update --recursive
cd targets/wasmtime/wasmtime
git submodule init
git submodule update --recursive
cd ../../..
if [ ! -d "coverage" ]; then
    mkdir coverage
fi
cd docker

# docker build --build-arg UID=$(id -u) --build-arg GID=$(id -g) -t fuzz-image .

export UID="$(id -u)"
export GID="$(id -g)"
docker-compose build
