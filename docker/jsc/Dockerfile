FROM rgfuzz-base

ARG UNAME=guest

# WORKDIR /fuzz/targets/jsc
# RUN git clone https://github.com/WebKit/WebKit.git --depth=1
# WORKDIR /fuzz

WORKDIR /fuzz/targets/jsc/WebKit
RUN git init
RUN git remote add origin https://github.com/WebKit/WebKit.git
RUN git fetch --depth=1 --no-tags origin cdf0e8ad7972a47c7ef9a620b28dcbc12eeb88f9
RUN git reset --hard cdf0e8ad7972a47c7ef9a620b28dcbc12eeb88f9
WORKDIR /fuzz

# buildroot
# WORKDIR /fuzz/targets/jsc
# RUN wget https://buildroot.org/downloads/buildroot-2023.08.2.tar.gz
# RUN tar xfa buildroot-2023.08.2.tar.gz
# WORKDIR /fuzz/targets/jsc/buildroot-2023.08.2
# RUN support/kconfig/merge_config.sh -n configs/raspberrypi3_64_defconfig ../jsc_buildroot_extraconfig
# RUN make -j $(nproc)

# JSC setup
COPY --chown=${UID}:${GID} targets/jsc /fuzz/targets/jsc
WORKDIR /fuzz/targets/jsc
RUN ./build-cov.sh
# RUN ./build.sh
WORKDIR /fuzz

COPY --chown=${UID}:${GID} fuzz /fuzz/fuzz
