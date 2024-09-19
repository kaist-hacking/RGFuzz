#!/bin/bash

python3 build-cov.py
cd v8
# export PATH=$PATH:/fuzz/depot_tools; python3 tools/dev/gm_modi.py x64.release arm64.release ia32.release arm.release loong64.release mips64el.release ppc64.release riscv32.release riscv64.release s390x.release
export PATH=$PATH:/fuzz/depot_tools; python3 tools/dev/gm_modi.py x64.release
cd ..
