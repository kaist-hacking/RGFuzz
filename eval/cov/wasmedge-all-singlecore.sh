#!/bin/bash

# make sure that the machine is running and the number of cores is large

export UID="$(id -u)"
export GID="$(id -g)"


eval_name=rgfuzz-wasmedge-stackgen-all-singlecore
num_processes=1
extractor_opt=all
generator_opt=stackgen
engine_name=wasmedge
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=rgfuzz-wasmedge-stackgen-typing-singlecore
num_processes=1
extractor_opt=typing
generator_opt=stackgen
engine_name=wasmedge
archs=x64

docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"


eval_name=xsmith-wasmedge-singlecore
num_processes=1
engine_name=wasmedge-xsmith
archs=x64

# run wasmedge rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --engine-name=$engine_name --archs=$archs"


eval_name=wasm-smith-wasmedge-singlecore
num_processes=1
extractor_opt=typing # unused
generator_opt=wasm-smith
engine_name=wasmedge
archs=x64

# run wasmedge rgfuzz with all settings
docker compose run --name "$eval_name-0" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-0 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-1" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-1 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-2" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-2 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-3" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-3 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
docker compose run --name "$eval_name-4" -u $(id -u) -di rgfuzz-wasmedge-single bash -c "source ~/.profile; python3 /fuzz/fuzz/eval/eval-wasmedge-rgfuzz-cov.py --eval-name=$eval_name-4 --num-processes=$num_processes --extractor-opt=$extractor_opt --generator-opt=$generator_opt --engine-name=$engine_name --archs=$archs"
