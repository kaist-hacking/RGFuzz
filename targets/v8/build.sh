#!/bin/bash

cd v8
tools/dev/gm.py x64.release arm64.release ia32.release arm.release loong64.release mips64el.release ppc64.release riscv32.release riscv64.release s390x.release
cd ..
