#!/bin/bash

# make sure that the machine is running and the number of cores is larger than 6*5=30

export UID="$(id -u)"
export GID="$(id -g)"


eval_name=rgfuzz-wasmtime-stackgen-all-singlecore
num_processes=1
extractor_opt=all
generator_opt=stackgen
engine_name=wasmtime
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=rgfuzz-wasmtime-stackgen-typing-singlecore
num_processes=1
extractor_opt=typing
generator_opt=stackgen
engine_name=wasmtime
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=wasmtime-differential-singlecore
num_processes=1

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-differential.py --eval-name=$eval_name-0 --num-processes=$num_processes"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-differential.py --eval-name=$eval_name-1 --num-processes=$num_processes"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-differential.py --eval-name=$eval_name-2 --num-processes=$num_processes"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-differential.py --eval-name=$eval_name-3 --num-processes=$num_processes"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-differential.py --eval-name=$eval_name-4 --num-processes=$num_processes"


eval_name=wasmtime-fuzzgen-singlecore
num_processes=1

# run wasmtime rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-fuzzgen.py --eval-name=$eval_name-0 --num-processes=$num_processes"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-fuzzgen.py --eval-name=$eval_name-1 --num-processes=$num_processes"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-fuzzgen.py --eval-name=$eval_name-2 --num-processes=$num_processes"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-fuzzgen.py --eval-name=$eval_name-3 --num-processes=$num_processes"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-fuzzgen.py --eval-name=$eval_name-4 --num-processes=$num_processes"


eval_name=wasmtime-wasm-mutate-singlecore
num_processes=1

# run wasmtime rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-wasm-mutate.py --eval-name=$eval_name-0 --num-processes=$num_processes"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-wasm-mutate.py --eval-name=$eval_name-1 --num-processes=$num_processes"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-wasm-mutate.py --eval-name=$eval_name-2 --num-processes=$num_processes"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-wasm-mutate.py --eval-name=$eval_name-3 --num-processes=$num_processes"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-wasm-mutate.py --eval-name=$eval_name-4 --num-processes=$num_processes"


eval_name=xsmith-wasmtime-singlecore
num_processes=1
engine_name=wasmtime-xsmith
archs=x64

# run wasmtime rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-xsmith-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-xsmith-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-xsmith-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-xsmith-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmtime-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmtime-xsmith-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
