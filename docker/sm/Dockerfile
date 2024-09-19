FROM rgfuzz-base

ARG UNAME=guest

# Download
WORKDIR /fuzz/targets/spidermonkey/gecko-dev
RUN git init
RUN git remote add origin https://github.com/mozilla/gecko-dev.git
RUN git fetch --depth=1 --no-tags origin c7df16ffad1f12a19c81c16bce0b65e4a15304d0
RUN git reset --hard c7df16ffad1f12a19c81c16bce0b65e4a15304d0
WORKDIR /fuzz

# SM setup
COPY --chown=${UID}:${GID} targets/spidermonkey /fuzz/targets/spidermonkey
WORKDIR /fuzz/targets/spidermonkey
RUN ./build-cov.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
