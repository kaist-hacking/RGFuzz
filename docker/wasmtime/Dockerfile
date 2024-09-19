FROM rgfuzz-base

ARG UNAME=guest

# wasmtime differential eval setting
COPY --chown=${UID}:${GID} targets/wasmtime/wasmtime /fuzz/targets/wasmtime/wasmtime
WORKDIR /fuzz/targets/wasmtime/wasmtime/fuzz
RUN eval $(opam env --switch=default); $HOME/.cargo/bin/cargo fuzz build --release
WORKDIR /fuzz

# xsmith wasmtime loader
WORKDIR /fuzz/targets/eval/xsmith/wasmtime/load-wasmtime
RUN RUSTFLAGS="-C instrument-coverage" $HOME/.cargo/bin/cargo build --release
WORKDIR /fuzz

# wasmtime wrapper
COPY --chown=${UID}:${GID} targets/wasmtime /fuzz/targets/wasmtime
WORKDIR /fuzz/targets/wasmtime
RUN ./build-cov.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
