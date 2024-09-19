import multiprocessing
import os
import logging, logging.handlers
import shutil
import sys
import time

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir
sys.path[0] = root_dir

import config
import codegen.generator as codegen
import engine

lock = multiprocessing.Lock()
worker_num_used_lock = multiprocessing.Lock()
worker_num_used_arr = multiprocessing.Array('b', [0]*config.num_processes)

bitmap_lock = multiprocessing.Lock()
virgin_bits = multiprocessing.Array('Q', ((1 << 64) - 1,)*1024*1024, lock=False)

class Fuzzer():
    def __init__(self, _engine, is_test=False):
        self.engine = _engine
        self.crash_cnt = 0
        self.interesting_cnt = 0
        self.is_init = False
        self.is_test = is_test

    # Factories
    @classmethod
    def get_fuzzer(cls, engine_name, is_test=False):
        if engine_name == "v8":
            return Fuzzer.get_v8_fuzzer(is_test)
        elif engine_name == "sm":
            return Fuzzer.get_sm_fuzzer(is_test)
        elif engine_name == "jsc":
            return Fuzzer.get_jsc_fuzzer(is_test)
        elif engine_name == "wasmtime":
            return Fuzzer.get_wasmtime_fuzzer(is_test)
        elif engine_name == "wasmer":
            return Fuzzer.get_wasmer_fuzzer(is_test)
        elif engine_name == "wasmedge":
            return Fuzzer.get_wasmedge_fuzzer(is_test)
        elif engine_name == "wasmtime-xsmith": # evaluation
            return Fuzzer.get_wasmtime_xsmith_fuzzer(is_test)
        elif engine_name == "wasmer-xsmith": # evaluation
            return Fuzzer.get_wasmer_xsmith_fuzzer(is_test)
        elif engine_name == "wasmedge-xsmith": # evaluation
            return Fuzzer.get_wasmedge_xsmith_fuzzer(is_test)
        elif engine_name == "v8-xsmith": # evaluation
            return Fuzzer.get_v8_xsmith_fuzzer(is_test)
        elif engine_name == "sm-xsmith": # evaluation
            return Fuzzer.get_sm_xsmith_fuzzer(is_test)
        elif engine_name == "jsc-xsmith": # evaluation
            return Fuzzer.get_jsc_xsmith_fuzzer(is_test)
        else:
            assert False # not yet implemented

    @classmethod
    def get_v8_fuzzer(cls, is_test=False):
        _engine = engine.V8Engine()
        return cls(_engine, is_test)

    @classmethod
    def get_sm_fuzzer(cls, is_test=False):
        _engine = engine.SMEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_jsc_fuzzer(cls, is_test=False):
        _engine = engine.JSCEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmtime_fuzzer(cls, is_test=False):
        _engine = engine.WasmtimeEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmer_fuzzer(cls, is_test=False):
        _engine = engine.WasmerEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmedge_fuzzer(cls, is_test=False):
        _engine = engine.WasmedgeEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmtime_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.WasmtimeXsmithEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmer_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.WasmerXsmithEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_wasmedge_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.WasmedgeXsmithEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_v8_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.V8XsmithEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_sm_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.SMXsmithEngine()
        return cls(_engine, is_test)

    @classmethod
    def get_jsc_xsmith_fuzzer(cls, is_test=False):
        _engine = engine.JSCXsmithEngine()
        return cls(_engine, is_test)

    # run
    def save_crash(self, result_raw):
        global lock

        lock.acquire()
        self.crash_cnt = 0
        if not os.path.exists(config.crash_save_rootdir):
            os.mkdir(config.crash_save_rootdir)

        while os.path.exists(os.path.join(config.crash_save_rootdir, "crash"+str(self.crash_cnt))):
            self.crash_cnt += 1

        crash_dir = os.path.join(config.crash_save_rootdir, "crash"+str(self.crash_cnt))
        os.mkdir(crash_dir)
        lock.release()

        files = os.listdir(self.workdir)
        for f in files:
            if ".cur." not in f:
                continue
            shutil.copyfile(os.path.join(self.workdir, f), os.path.join(crash_dir, f[5:]))

        for (arch, opt_level_str) in result_raw.keys():
            exec_result, exec_result2, is_crash = result_raw[(arch, opt_level_str)]
            with open(os.path.join(crash_dir, f".cur.exec.{arch}.{opt_level_str}"), "wb") as f:
                f.write(exec_result+b'\n'+exec_result2+b'\n'+(b'crash' if is_crash else b'done'))

        # TODO: cross-engine testing
        with open(os.path.join(crash_dir, "NOTE"), "wt") as f:
            f.write(str(self.engine))

    def init_fuzzer(self):
        global worker_num_used_arr
        global worker_num_used_lock

        # get wid for logging
        self.wid = -1
        while self.wid == -1:
            worker_num_used_lock.acquire()
            for idx, is_used in enumerate(worker_num_used_arr):
                if is_used == 0:
                    self.wid = idx
                    worker_num_used_arr[idx] = 1
                    break
            worker_num_used_lock.release()
            time.sleep(0.5)

        try:
            os.mkdir(config.workdir)
        except FileExistsError:
            pass

        if self.is_test:
            self.workdir = os.path.join(config.workdir, "test")
        else:
            self.workdir = os.path.join(config.workdir, "work"+str(self.wid))

        if os.path.exists(self.workdir):
            assert not os.path.isfile(self.workdir)
            shutil.rmtree(self.workdir)
        os.mkdir(self.workdir)

        log_file = os.path.join(self.workdir, "log")
        log_handler = logging.handlers.RotatingFileHandler(
          log_file, mode="a", maxBytes=config.log_file_limit,
          backupCount=config.log_backup_cnt, encoding=None
        )
        self.log_formatter = logging.Formatter("%(asctime)-23s | %(name)s | %(levelname)-8s | %(message)s")
        log_handler.setFormatter(self.log_formatter)
        self.logger = logging.getLogger("fuzz")
        self.logger.addHandler(log_handler)

        self.cur_log_file = os.path.join(self.workdir, ".cur.log")
        self.cur_log_handler = logging.FileHandler(self.cur_log_file, mode="a")
        self.cur_log_handler.setFormatter(self.log_formatter)
        self.logger.addHandler(self.cur_log_handler)

        self.is_init = True


    def fini_fuzzer(self):
        worker_num_used_lock.acquire()
        assert worker_num_used_arr[self.wid] == 1
        worker_num_used_arr[self.wid] = 0
        worker_num_used_lock.release()

        self.cur_log_handler.close()
        self.logger.removeHandler(self.cur_log_handler)

        self.is_init = False


    def fuzz(self, seed):
        global worker_num_used_arr
        global worker_num_used_lock

        assert self.is_init

        # reset logging for current log
        self.cur_log_handler.flush()
        open(self.cur_log_file, 'w').close()

        self.logger.warning("[*] Seed: " + str(seed))

        # prepare feedbacks (unused)
        feedbacks = {}
        for arch in self.engine.arch_list:
            feedbacks[arch] = None

        # run
        results = []
        code, itypes, otypes, instrs = self.engine.get_run_code(seed, self.workdir)
        instr_ast = codegen.describe_instrs(instrs)
        self.logger.warning("[*] Executing " + str(instr_ast))
        compile_success, compile_fail_archs, compare_result = self.run(code, instr_ast, feedbacks, seed)
        results.append((seed, compile_success, compile_fail_archs, compare_result, instr_ast))

        return results

    def run(self, code, instr_ast, feedbacks, seed):
        with open(os.path.join(self.workdir, ".cur.input"), "wb") as f:
            f.write(code)

        # compare
        compile_success, compile_success_archs, compare_result, is_crash, _, results_raw = self.engine.do_compare(self.workdir, feedbacks, seed)
        if not compare_result or is_crash or (not compile_success and config.catch_compile_error):
            print("[!] Crash found in process " + str(self.wid))
            print("[!] Instruction AST:", instr_ast)

            # log result
            if not compare_result and not is_crash:
                self.logger.error(f"[!] Detected with mismatch")
            elif is_crash:
                self.logger.error("[!] Detected with crash")

            # flush doesn't work
            self.cur_log_handler.close()
            self.logger.removeHandler(self.cur_log_handler)
            if not self.is_test and config.save_crashes:
                self.save_crash(results_raw)

            self.cur_log_handler = logging.FileHandler(self.cur_log_file, mode="a")
            self.cur_log_handler.setFormatter(self.log_formatter)
            self.logger.addHandler(self.cur_log_handler)

        compile_fail_archs = []
        for cl in self.engine.compare_list:
            if cl not in compile_success_archs:
                compile_fail_archs.append(cl)

        return compile_success, compile_fail_archs, compare_result
