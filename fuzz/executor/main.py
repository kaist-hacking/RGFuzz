#!/usr/bin/env python3

import logging
import multiprocessing, multiprocessing.util
import os
import sys
import time

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir
sys.path[0] = root_dir

import fuzzer
import config
from codegen import genstream

main_kill_switch = False
fuzz_inst = fuzzer.Fuzzer.get_fuzzer(config.engine_name)

def init():
    global fuzz_inst
    fuzz_inst.init_fuzzer()
    multiprocessing.util.Finalize(fuzz_inst, fini, exitpriority=-1)

def fini(*args, **kwargs):
    global fuzz_inst
    fuzz_inst.fini_fuzzer()

def run(seed):
    global fuzz_inst
    return fuzz_inst.fuzz(seed)

def main():
    global main_kill_switch
    global fuzz_inst

    logging.getLogger().handlers.clear()
    logger = logging.getLogger("fuzz")
    logger.setLevel(config.logging_level)
    logger.propagate = False

    gen_iter = genstream.code_seed_gen
    start_time = time.time()

    with multiprocessing.Pool(
      config.num_processes,
      initializer=init,
      maxtasksperchild=config.max_tasks_per_child
    ) as pool:
        pool_iter = pool.imap_unordered(run, gen_iter, config.multiprocessing_chunksize)
        for idx, results in enumerate(pool_iter):
            if idx % config.main_print_frequency == 0:
                for result in results:
                    elapsed_time = time.time() - start_time
                    seed, compile_success, compile_fail_archs, compare_result, instr_ast = result
                    info = f"[*] {idx+config.codegen_seed_enum_start_from}: "
                    info += f"seed: {seed} / "
                    info += f"execution speed: {(idx+1)/elapsed_time}execs/s / "
                    info += f"compile success: {compile_success} / "
                    info += f"compile failing archs: {compile_fail_archs} / "
                    info += f"compare result: {compare_result} / "
                    info += f"instructions: {instr_ast}"
                    print(info)

            if main_kill_switch:
                pool.close()
                pool.terminate()
                pool.join()
                return
        pool.close()
        pool.join()

if __name__ == "__main__":
    main()