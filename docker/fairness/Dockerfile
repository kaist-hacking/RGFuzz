FROM rgfuzz-base

ARG UNAME=guest

WORKDIR /fuzz/targets/wasmtime/wasmtime
RUN $HOME/.cargo/bin/cargo build --release
WORKDIR /fuzz

# IR inspector
COPY --chown=${UID}:${GID} fuzz/eval/ir-inspector /fuzz/fuzz/eval/ir-inspector
WORKDIR /fuzz/fuzz/eval/ir-inspector
RUN $HOME/.cargo/bin/cargo build --release
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
