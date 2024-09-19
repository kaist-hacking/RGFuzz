#!/usr/bin/env python3

import os
import sys
import subprocess
import random
import shutil
import multiprocessing.pool
import re

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.join(os.path.dirname(source_dir), "executor")
sys.path[0] = root_dir

import config
import codegen.instructions
import codegen.generator

NUM_ITERATIONS = 100000
SEED_LEN = 4096
TESTDIR = os.path.join(config.workdir, "test")
GEN = codegen.generator.XsmithWrapper()

def run_generator(seed):
    code = GEN.get_run_code(seed, TESTDIR)[0]
    
    subp = subprocess.Popen(
        ["wasm-tools", "parse", "-t", "-"],
        shell=False,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    result, err = subp.communicate(input=code)
    subp.kill()
    
    return code, is_module_valid(code) and has_func(result)

def has_func(wat):
    return b"(func " in wat

def is_module_valid(code):
    subp = subprocess.Popen(
        ["wasm-tools", "validate", "--features=all", "-"],
        shell=False,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    result, err = subp.communicate(input=code)
    subp.kill()

    return len(err) == 0

def count_irexpr_number(irexpr_counts, code):
    subp = subprocess.Popen(
        [
            os.path.join(os.path.dirname(root_dir), "eval", "ir-inspector", "target", "release", "ir-inspector"),
            "/dev/stdin"
        ],
        shell=False,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    result, err = subp.communicate(input=code)
    subp.kill()
    
    result_counts = eval(result)
    for op, cnt in result_counts.items():
        if op in irexpr_counts:
            irexpr_counts[op] += cnt
        else:
            irexpr_counts[op] = cnt

def emit_irexpr_counts(target_dir, irexpr_counts):
    with open(os.path.join(target_dir, "counts.txt"), "wt") as f:
        f.write(str(irexpr_counts))
    
    total_count = sum(irexpr_counts.values())
    irexpr_freqs = {op: cnt / total_count for op, cnt in irexpr_counts.items()}
    with open(os.path.join(target_dir, "freqs.txt"), "wt") as f:
        f.write(str(irexpr_freqs))

if __name__ == "__main__":
    if not os.path.exists(TESTDIR):
        os.makedirs(TESTDIR)

    module_valid = 0
    irexpr_counts = {}
    cnt = 0
    with multiprocessing.pool.ThreadPool(
        config.num_processes
    ) as pool:
        pool_iter = pool.imap_unordered(
            run_generator, 
            (random.randint(0, 2**(config.codegen_seed_len*8)-1) for _ in range(NUM_ITERATIONS))
        )
        for code, module_valid_bool in pool_iter:
            cnt += 1
            if cnt % 1000 == 0:
                print(f"Processed {cnt} modules")
            if module_valid_bool:
                module_valid += 1
            count_irexpr_number(irexpr_counts, code)
    
    emit_irexpr_counts(TESTDIR, irexpr_counts)

    # save to coverage path
    report_dir = os.path.join(config.coverage_dir)
    if not os.path.exists(report_dir):
        os.makedirs(report_dir)
    else:
        shutil.rmtree(report_dir)
        os.makedirs(report_dir)
    emit_irexpr_counts(report_dir, irexpr_counts)
