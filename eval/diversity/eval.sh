#!/bin/bash

export UID="$(id -u)"
export GID="$(id -g)"

cd ../../docker/

eval_name=wasm-smith-fairness
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasm-smith-fairness.py --no-canonicalize-nans --eval-name=$eval_name"

eval_name=rgfuzz-fairness-all
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-rgfuzz-fairness.py --extractor-opt=all --eval-name=$eval_name"

eval_name=xsmith-fairness
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-$eval_name.py --eval-name=$eval_name"

eval_name=wasm-smith-ir-fairness
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasm-smith-ir-fairness.py --no-canonicalize-nans --eval-name=$eval_name"

eval_name=rgfuzz-ir-fairness-all
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-rgfuzz-ir-fairness.py --extractor-opt=all --eval-name=$eval_name"

eval_name=xsmith-ir-fairness
docker compose run --name $eval_name -u $(id -u) -di rgfuzz-fairness bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-$eval_name.py --eval-name=$eval_name"

cd ../eval/diversity