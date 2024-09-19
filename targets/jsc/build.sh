#!/bin/bash

cd WebKit
export WEBKIT_OUTPUTDIR="$(pwd)/builds/x64"
Tools/Scripts/build-webkit --jsc-only --release

# export BR2_HOST_DIR="$(pwd)/buildroot-2023.08.2/output/host"
# export ICU_ROOT="${BR2_HOST_DIR}"
# export PKG_CONFIG_PATH="${BR2_HOST_DIR}/lib/pkgconfig"
# export CROSS_COMPILE="$(basename $(cat ${BR2_HOST_DIR}/usr/share/buildroot/toolchainfile.cmake|grep CMAKE_CXX_COMPILER|awk -F'"' '{print $2}')|sed "s/g++$//g")"
# export PATH="${BR2_HOST_DIR}/usr/bin:${PATH}"
# export CC="${CROSS_COMPILE}gcc"
# export CXX="${CROSS_COMPILE}g++"
# export LINK="${CROSS_COMPILE}g++"
# export LINK_SHLIB="${CROSS_COMPILE}g++"
# export AR="${CROSS_COMPILE}ar"
# export OBJCOPY="${CROSS_COMPILE}objcopy"
# export STRIP="${CROSS_COMPILE}strip"
# export BUILD_WEBKIT_ARGS="--cmakeargs=-DCMAKE_TOOLCHAIN_FILE=${BR2_HOST_DIR}/usr/share/buildroot/toolchainfile.cmake"
# export WEBKIT_OUTPUTDIR="$(pwd)/builds/arm64"
# Tools/Scripts/build-webkit --jsc-only --release
cd ..