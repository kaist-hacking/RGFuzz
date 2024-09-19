FROM rgfuzz-base

ARG UNAME=guest

# FUTURE WORK

# WAMR: because of lack of documentation, this leaves as a FUTURE WORK
# COPY --chown=${UID}:${GID} targets/wamr/wasm-micro-runtime /fuzz/targets/wamr/wasm-micro-runtime
# WORKDIR /fuzz/targets/wamr/wasm-micro-runtime/language-bindings/python
# RUN python3 -m pip install -e .
# WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
