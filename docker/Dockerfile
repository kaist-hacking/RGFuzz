FROM ubuntu:22.04

LABEL maintainer="parkjuny@kaist.ac.kr"
LABEL version="0.1"
LABEL description="Docker image for rgfuzz"

ARG UNAME=guest
ARG UID
ARG GID

ENV UID $UID
ENV GID $GID

RUN groupadd -g $GID -o $UNAME \
    && useradd -m -u $UID -g $GID -o -s /bin/bash $UNAME

RUN apt update
RUN apt upgrade -y
RUN apt dist-upgrade -y
RUN apt autoremove -y

# liblld-14-dev not needed - remove later
RUN apt install -y python3 python3-pip git ninja-build pkg-config libglib2.0-dev libfdt-dev libpixman-1-dev zlib1g-dev flex bison curl gcc-aarch64-linux-gnu gcc-riscv64-linux-gnu gcc-s390x-linux-gnu opam libgmp-dev lsb-release sudo lib32gcc-11-dev libx32gcc-11-dev wget llvm cpio bc vim racket wabt libssl-dev cmake liblld-14-dev ruby-full
RUN DEBIAN_FRONTEND=noninteractive apt install -y gcovr
# RUN apt install -y libclang-common-15-dev llvm-15

# llvm-14 is broken for profile merge, so installing this
RUN echo 'deb http://apt.llvm.org/jammy/ llvm-toolchain-jammy-19 main' >> /etc/apt/sources.list
RUN echo 'deb-src http://apt.llvm.org/jammy/ llvm-toolchain-jammy-19 main' >> /etc/apt/sources.list
RUN wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
RUN apt update
RUN apt install -y llvm-19 libboost-all-dev clang-19 liblld-19-dev llvm-19-linker-tools
RUN DEBIAN_FRONTEND=noninteractive apt-get install keyboard-configuration

RUN pip3 install ctypesgen sysv_ipc overrides

USER $UNAME

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    && echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

COPY --chown=${UID}:${GID} targets/wasmtime/wasmtime-wrapper/.cargo $HOME/.cargo
RUN $HOME/.cargo/bin/rustup default nightly
RUN $HOME/.cargo/bin/rustup target add aarch64-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup target add riscv64gc-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup target add s390x-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup target add i686-unknown-linux-gnu
RUN $HOME/.cargo/bin/rustup component add llvm-tools
RUN $HOME/.cargo/bin/cargo install cargo-fuzz wasm-tools cargo-binutils llvmenv

RUN raco pkg install --auto clotho; exit 0
RUN raco pkg install --auto xsmith; exit 0

RUN opam init -y
RUN opam install -y ocamlfind zarith=1.12 ocamlbuild

RUN echo "eval \$(opam env --switch=default)" >> ~/.bashrc
RUN echo "eval \$(opam env --switch=default)" >> ~/.profile

WORKDIR /fuzz/qemu
RUN git init
RUN git remote add origin https://github.com/qemu/qemu.git
RUN git fetch --depth=1 --no-tags origin tag v9.1.0
RUN git reset --hard v9.1.0
WORKDIR /fuzz/qemu/build
RUN pip3 install tomli
RUN ../configure --target-list=aarch64-linux-user,riscv64-linux-user,s390x-linux-user
RUN make -j $(nproc)
USER root
RUN make install
USER $UNAME
WORKDIR /fuzz

# xsmith
COPY --chown=${UID}:${GID} targets/eval/xsmith /fuzz/targets/eval/xsmith
WORKDIR /fuzz/targets/eval/xsmith/wasmlike
RUN raco make wasmlike.rkt
WORKDIR /fuzz

# extractor
COPY --chown=${UID}:${GID} targets/wasmtime/wasmtime /fuzz/targets/wasmtime/wasmtime
COPY --chown=${UID}:${GID} fuzz/extractor /fuzz/fuzz/extractor
WORKDIR /fuzz/fuzz/extractor
RUN $HOME/.cargo/bin/cargo build --release
WORKDIR /fuzz
