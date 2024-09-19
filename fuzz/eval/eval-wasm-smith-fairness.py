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
INSTR_OPCODES = [x for x in codegen.instructions.INSTRUCTIONS if 'atomic' not in x]

def run_generator(seed):
    seed_bytes = random.Random(seed).randbytes(SEED_LEN)
    
    args = [
        "wasm-tools", "smith", 
        "--min-types", "1", 
        "--min-funcs", "1", 
        "--min-memories", "1", 
        "--max-modules", "1", 
        "--simd-enabled", "true", 
        "--export-everything", "true",
        "--max-imports", "0",
        "--max-data-segments", "0",
        "--max-element-segments", "0",
        "--allow-start-export", "false",
        "--canonicalize-nans", "true" if config.wasm_smith_canon_nans else "false"
    ]
    subp = subprocess.Popen(
        args, shell=False,
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
    )
    code, err = subp.communicate(input=seed_bytes)
    subp.kill()
    
    subp = subprocess.Popen(
        ["wasm-tools", "parse", "-t", "-"],
        shell=False,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    result, err = subp.communicate(input=code)
    subp.kill()
    
    return result, is_module_valid(code) and has_func(result)

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

def count_instr_number(instr_counts, wat):
    for idx, op in enumerate(INSTR_OPCODES):
        instr_counts[idx] += wat.count(b' '+op.encode()+b' ')
        instr_counts[idx] += wat.count(b' '+op.encode()+b'\n')

def extract_blocktypes(line):
    param_pattern = r'\(param\s+(.*?)\)'
    result_pattern = r'\(result\s+(.*?)\)'

    param_match = re.search(param_pattern, line.decode())
    result_match = re.search(result_pattern, line.decode())

    if param_match:
        param_types = tuple(param_match.group(1).encode().split(b' '))
    else:
        param_types = ()
    
    if result_match:
        result_types = tuple(result_match.group(1).encode().split(b' '))
    else:
        result_types = ()
    
    return param_types, result_types

# (blocktype, (params, rets)) => count
def count_block_number(block_counts, wat):
    for wat_line in wat.split(b'\n'):
        wat_line_strip = wat_line.strip()
        if wat_line_strip.startswith(b'block'):
            block_types = extract_blocktypes(wat_line)
            key = (b'block', block_types[0], block_types[1])
            if not key in block_counts:
                block_counts[key] = 1
            else:
                block_counts[key] += 1
        if wat_line_strip.startswith(b'loop'):
            block_types = extract_blocktypes(wat_line)
            key = (b'loop', block_types[0], block_types[1])
            if not key in block_counts:
                block_counts[key] = 1
            else:
                block_counts[key] += 1
        if wat_line_strip.startswith(b'if'):
            block_types = extract_blocktypes(wat_line)
            key = (b'if', block_types[0], block_types[1])
            if not key in block_counts:
                block_counts[key] = 1
            else:
                block_counts[key] += 1

def emit_instr_counts(instr_counts):
    with open(os.path.join(TESTDIR, "counts.txt"), "wt") as f:
        f.write(str(list(zip(INSTR_OPCODES, instr_counts))))
    
    total_count = sum(instr_counts)
    instr_freqs = [x / total_count for x in instr_counts]
    with open(os.path.join(TESTDIR, "freqs.txt"), "wt") as f:
        f.write(str(list(zip(INSTR_OPCODES, instr_freqs))))

if __name__ == "__main__":
    if not os.path.exists(TESTDIR):
        os.makedirs(TESTDIR)

    module_valid = 0
    instr_counts = [0 for _ in range(len(INSTR_OPCODES))]
    block_counts = {} # (blocktype, (params, rets)) => count
    cnt = 0
    with multiprocessing.pool.ThreadPool(
        config.num_processes
    ) as pool:
        pool_iter = pool.imap_unordered(
            run_generator, 
            (random.randint(0, 2**(config.codegen_seed_len*8)-1) for _ in range(NUM_ITERATIONS))
        )
        for wat, module_valid_bool in pool_iter:
            cnt += 1
            if cnt % 1000 == 0:
                print(f"Processed {cnt} modules")
            if module_valid_bool:
                module_valid += 1
            count_instr_number(instr_counts, wat)
            count_block_number(block_counts, wat)
    
    emit_instr_counts(instr_counts)
    print("MODULE VALID COUNT:", module_valid)

    # save to coverage path
    report_dir = os.path.join(config.coverage_dir)
    if not os.path.exists(report_dir):
        os.makedirs(report_dir)
    else:
        shutil.rmtree(report_dir)
        os.makedirs(report_dir)
    
    with open(os.path.join(report_dir, "counts.txt"), "wt") as f:
        f.write(str(list(zip(INSTR_OPCODES, instr_counts))))
        f.write('\n'*2)
        f.write(str(block_counts))
        f.write('\n'*2)
        f.write(f"\nModule valid count: {module_valid}\n")

    total_count = sum(instr_counts)
    total_block_count = sum(block_counts.values())
    instr_freqs = [x / total_count for x in instr_counts]
    block_freqs = [(x, y / total_block_count) for x, y in block_counts.items()]
    with open(os.path.join(report_dir, "freqs.txt"), "wt") as f:
        f.write(str(list(zip(INSTR_OPCODES, instr_freqs))))
        f.write('\n'*2)
        f.write(str(block_freqs))
        f.write('\n'*2)
        f.write(f"\nModule valid count: {module_valid}\n")
