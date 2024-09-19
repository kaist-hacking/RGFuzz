FROM rgfuzz-base

ARG UNAME=guest

# Download
RUN git clone https://chromium.googlesource.com/chromium/tools/depot_tools.git
WORKDIR /fuzz/targets/v8
RUN /fuzz/depot_tools/fetch --no-history --no-hooks v8
WORKDIR /fuzz/targets/v8/v8
RUN git fetch origin tag 12.6.21 --no-tags --depth=1
# RUN git fetch --tags
RUN git reset --hard 12.6.21
# RUN rm -rf /fuzz/targets/v8/.cipd
RUN export PATH=$PATH:/fuzz/depot_tools; gclient sync
USER root
RUN build/install-build-deps.sh
USER $UNAME
WORKDIR /fuzz

# # V8 setup
COPY --chown=${UID}:${GID} targets/v8 /fuzz/targets/v8
WORKDIR /fuzz/targets/v8
RUN ./build-cov.sh
# # RUN ./build.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
