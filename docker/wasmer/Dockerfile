FROM rgfuzz-base

ARG UNAME=guest

# llvm for wasmer llvm
WORKDIR /fuzz/targets/wasmer/llvm-15
RUN git init
RUN git remote add origin https://github.com/llvm/llvm-project.git
RUN git fetch --depth=1 --no-tags origin tag llvmorg-15.0.7
RUN git reset --hard llvmorg-15.0.7
RUN cmake -S llvm -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=clang++-19 -DCMAKE_C_COMPILER=clang-19 -DLLVM_BUILD_INSTRUMENTED_COVERAGE=ON -DLLVM_USE_LINKER=lld-19
WORKDIR /fuzz/targets/wasmer/llvm-15/build
RUN make -j
WORKDIR /fuzz

# xsmith wasmer loader
WORKDIR /fuzz/targets/eval/xsmith/wasmer/load-wasmer
RUN LLVM_SYS_150_PREFIX=/fuzz/targets/wasmer/llvm-15/build RUSTFLAGS="-C instrument-coverage" $HOME/.cargo/bin/cargo build --release
WORKDIR /fuzz

# wasmer wrapper
COPY --chown=${UID}:${GID} targets/wasmer /fuzz/targets/wasmer
WORKDIR /fuzz/targets/wasmer
RUN ./build-cov.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
