#!/bin/bash

export MOZ_FETCHES_DIR="$HOME/.mozbuild"
mkdir builds
cd gecko-dev
export MOZCONFIG=$PWD/../mozconfigs/release.x64
./mach build
mv obj-x86_64-pc-linux-gnu ../builds/x64_release
export MOZCONFIG=$PWD/../mozconfigs/coverage.x64
./mach build
mv obj-x86_64-pc-linux-gnu ../builds/x64_coverage
export MOZCONFIG=$PWD/../mozconfigs/release.arm64
./mach build
mv obj-x86_64-pc-linux-gnu ../builds/arm64_release
export MOZCONFIG=$PWD/../mozconfigs/coverage.arm64
./mach build
mv obj-x86_64-pc-linux-gnu ../builds/arm64_coverage
export MOZCONFIG=$PWD/../mozconfigs/release.ia32
./mach build
mv obj-i686-pc-linux-gnu ../builds/ia32_release
export MOZCONFIG=$PWD/../mozconfigs/coverage.ia32
./mach build
mv obj-i686-pc-linux-gnu ../builds/ia32_coverage
export MOZCONFIG=$PWD/../mozconfigs/release.arm
./mach build
mv obj-i686-pc-linux-gnu ../builds/arm_release
export MOZCONFIG=$PWD/../mozconfigs/coverage.arm
./mach build
mv obj-i686-pc-linux-gnu ../builds/arm_coverage
# export MOZCONFIG=$PWD/../mozconfigs/release.mips32
# ./mach build
# mv obj-i686-pc-linux-gnu ../builds/mips32_release
# export MOZCONFIG=$PWD/../mozconfigs/coverage.mips32
# ./mach build
# mv obj-i686-pc-linux-gnu ../builds/mips32_coverage
# export MOZCONFIG=$PWD/../mozconfigs/release.mips64
# ./mach build
# mv obj-x86_64-pc-linux-gnu ../builds/mips64_release
# export MOZCONFIG=$PWD/../mozconfigs/coverage.mips64
# ./mach build
# mv obj-x86_64-pc-linux-gnu ../builds/mips64_coverage
# export MOZCONFIG=$PWD/../mozconfigs/release.loong64
# ./mach build
# mv obj-x86_64-pc-linux-gnu ../builds/loong64_release
# export MOZCONFIG=$PWD/../mozconfigs/coverage.loong64
# ./mach build
# mv obj-x86_64-pc-linux-gnu ../builds/loong64_coverage
cd ..