FROM rgfuzz-base

ARG UNAME=guest

USER root
RUN apt install -y libzstd-dev
USER $UNAME

# llvm for wasmedge (no openmp)
WORKDIR /fuzz/targets/wasmedge/llvm-16
RUN git init
RUN git remote add origin https://github.com/llvm/llvm-project.git
RUN git fetch --depth=1 --no-tags origin tag llvmorg-16.0.6
RUN git reset --hard llvmorg-16.0.6
RUN cmake -S llvm -B build -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=clang++-19 -DCMAKE_C_COMPILER=clang-19 -DLLVM_BUILD_INSTRUMENTED_COVERAGE=ON -DLLVM_USE_LINKER=lld-19 -DLLVM_ENABLE_PROJECTS="clang;clang-tools-extra;cross-project-tests;libc;libclc;lld;lldb;polly;pstl"
RUN cmake --build build -j
RUN cmake --install build --prefix /fuzz/llvm
WORKDIR /fuzz

WORKDIR /fuzz/targets/wasmedge/WasmEdge
RUN git init
RUN git remote add origin https://github.com/WasmEdge/WasmEdge
RUN git fetch --depth=1 --no-tags origin tag 0.13.5
RUN git reset --hard 0.13.5
WORKDIR /fuzz/targets/wasmedge
COPY --chown=${UID}:${GID} targets/wasmedge/CodeCoverage.cmake /fuzz/targets/wasmedge/WasmEdge/cmake/CodeCoverage.cmake
WORKDIR /fuzz/targets/wasmedge/WasmEdge/build
RUN cmake -DLLVM_DIR=/fuzz/llvm/build -DLLVM_BINARY_DIR=/fuzz/llvm/bin -DLLVM_INCLUDE_DIR=/fuzz/llvm/include -DLLVM_CMAKE_PATH=/fuzz/llvm/lib/cmake -DCMAKE_BUILD_TYPE=Release -DWASMEDGE_BUILD_TESTS=OFF -DWASMEDGE_BUILD_COVERAGE=ON -DCMAKE_C_COMPILER=clang-19 -DCMAKE_CXX_COMPILER=clang++-19 -DCMAKE_CXX_FLAGS="-Wno-deprecated-declarations" .. && make -j

# wasmedge wrapper
COPY --chown=${UID}:${GID} targets/wasmedge /fuzz/targets/wasmedge
WORKDIR /fuzz/targets/wasmedge
RUN ./build-cov.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz

# RUN: LD_LIBRARY_PATH=$PWD/../WasmEdge/build/lib/api