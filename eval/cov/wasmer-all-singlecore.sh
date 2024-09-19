#!/bin/bash

# make sure that the machine is running and the number of cores is large

export UID="$(id -u)"
export GID="$(id -g)"


eval_name=rgfuzz-wasmer-stackgen-all-singlecore
num_processes=1
extractor_opt=all
generator_opt=stackgen
engine_name=wasmer
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=rgfuzz-wasmer-stackgen-typing-singlecore
num_processes=1
extractor_opt=typing
generator_opt=stackgen
engine_name=wasmer
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=xsmith-wasmer-singlecore
num_processes=1
engine_name=wasmer-xsmith
archs=x64

# run wasmer rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-xsmith-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-xsmith-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-xsmith-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-xsmith-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-xsmith-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"


eval_name=wasm-smith-wasmer-singlecore
num_processes=1
extractor_opt=typing # unused
generator_opt=wasm-smith
engine_name=wasmer
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmer-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmer-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
