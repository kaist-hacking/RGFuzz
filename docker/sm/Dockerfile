FROM rgfuzz-base

ARG UNAME=guest

# Install clang-18 - clang-19 errors
# Also install lib32gcc-12-dev for 32-bit builds (version 11 does not work)
USER root
RUN echo 'deb http://apt.llvm.org/jammy/ llvm-toolchain-jammy-18 main' >> /etc/apt/sources.list
RUN echo 'deb-src http://apt.llvm.org/jammy/ llvm-toolchain-jammy-18 main' >> /etc/apt/sources.list
RUN dpkg --add-architecture i386
RUN apt update
RUN apt install -y clang-18 libstdc++-11-dev g++-multilib libc6-dev-i386 zlib1g-dev:i386
USER $UNAME
RUN $HOME/.cargo/bin/cargo install cbindgen

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
