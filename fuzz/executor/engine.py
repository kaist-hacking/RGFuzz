from overrides import override
import os
import sys
import itertools
import logging
import hashlib

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir
sys.path[0] = root_dir

import config
import executor
from codegen import generator, stackgen

class Engine():
    def __init__(self):
        self.arch_list = []
        self.opt_range = []
        self.executor_list = []
        self.compare_list = []
        self.generator = None

    def __str__(self):
        return "Engine"

    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        return "default"

    def str_to_opt_level(self, opt_str):
        return 0

    # codegen
    def filter_test_case(self, code):
        return True

    # run
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        return self.generator.get_run_code(seed, workdir, instrs_overwrite)

    def postprocess_exec_result(self, exec_result):
        exec_result_split = exec_result.split(b'\n')
        res_split = []
        for line in exec_result_split:
            if line.startswith(b'RuntimeError: '):
                res_split.append(b'RuntimeError')
            else:
                res_split.append(line)
        return b'\n'.join(res_split)

    def do_compare(self, workdir, feedbacks, seed):
        logger = logging.getLogger("fuzz")

        results = []
        results_raw = {}
        compile_success = True
        compile_success_archs = []
        is_crash_total = False

        for cl in self.compare_list:
            arch = cl[0]
            opt_level_str= self.opt_level_to_str(cl[1])
            code_path = os.path.join(workdir, ".cur.input")
            exec_result, exec_result2, returncode = self.executor_list[cl[1]].run(workdir, code_path, arch, feedbacks[arch], seed)

            is_crash = returncode != None and returncode < 0
            is_crash_total |= is_crash

            results_raw[(arch, opt_level_str)] = (exec_result, exec_result2, is_crash)

            exec_result_post = self.postprocess_exec_result(exec_result)
            classified_result = self.executor_list[cl[1]].classify_result(exec_result_post)
            if classified_result == 'Success' and not is_crash:
                results.append(exec_result_post)
                compile_success_archs.append(cl)
                exec_result_post_hash = hashlib.md5(exec_result_post).hexdigest()
                logger.warning(f"[*] {arch}-{opt_level_str} exec result: {exec_result_post_hash}, {returncode}")
            elif classified_result == 'CompileError':
                compile_success = False
                logger.warning(f"[*] {arch}-{opt_level_str} compile failed: {returncode}")
            elif classified_result == 'RunError':
                compile_success = False
                logger.warning(f"[*] {arch}-{opt_level_str} run failed: {returncode}")
            elif classified_result == 'Timeout':
                logger.warning(f"[*] {arch}-{opt_level_str} timed out: {returncode}")
            elif is_crash:
                logger.warning(f"[*] {arch}-{opt_level_str} crashed: {returncode}")
            else:
                assert False

        is_success = len(set(results)) <= 1
        return compile_success, compile_success_archs, is_success, is_crash_total, results, results_raw


class V8Engine(Engine):
    def __init__(self):
        super().__init__()

        liftoff_exec = executor.LiftoffExecutor(config.v8_additional_configs)
        turbofan_exec = executor.TurboFanExecutor(config.v8_additional_configs)

        self.arch_list=config.v8_arch_list
        self.opt_range=range(2)
        self.executor_list=[liftoff_exec, turbofan_exec]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.JSWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithJSWrapper()
        else:
            assert False

        self.compare_list = list(itertools.product(config.v8_arch_list, self.opt_range))

    def __str__(self):
        return "v8"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        return "liftoff" if opt_level == 0 else "turbofan"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "liftoff":
            return 0
        elif opt_str == "turbofan":
            return 1
        else:
            return -1

    # codegen
    def code_filter(self, code_line):
        # if b"AtomicCompareExchange" in code_line:
        #   return True
        # elif b"f32" in code_line or b"f64" in code_line: # TODO: remove this (TEMP)
        #   return True
        # elif b"ctz" in code_line:
        #   return True
        # else:
        #   return False
        return False

    @override
    def filter_test_case(self, code):
        code_split = code.split(b"\n")
        func_start = [i for i, v in enumerate(code_split) if v.startswith(b"builder.addFunction(undefined")][0]
        func_end = [i for i, v in enumerate(code_split[func_start:]) if v == b"]);"][0] + func_start
        return not any([self.code_filter(i) for i in code_split[func_start:func_end]])

    @override
    def postprocess_exec_result(self, exec_result):
        exec_result_split = exec_result.split(b'\n')
        res_split = []
        skip_idx = -1
        for idx in range(len(exec_result_split)):
            if idx < skip_idx:
                continue

            line = exec_result_split[idx]
            if line.startswith(b'wasm-function['):
                skip_idx = idx+5 # skip entire runtime error context
                res_split.append(b'RuntimeError')
            elif line.startswith(b'RuntimeError: '):
                res_split.append(b'RuntimeError')
            else:
                res_split.append(line)
        return b'\n'.join(res_split)


class SMEngine(Engine):
    def __init__(self):
        super().__init__()

        baseline_exec = executor.BaselineExecutor(config.sm_additional_configs)
        ion_exec = executor.IonExecutor(config.sm_additional_configs)

        self.arch_list=config.sm_arch_list
        self.opt_range=range(2)
        self.executor_list=[baseline_exec, ion_exec]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.JSWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithJSWrapper()
        else:
            assert False

        self.compare_list = list(itertools.product(config.sm_arch_list, self.opt_range))

    def __str__(self):
        return "sm"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        return "baseline" if opt_level == 0 else "ion"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "baseline":
            return 0
        elif opt_str == "ion":
            return 1
        else:
            return -1

    # codegen
    def code_filter(self, code_line):
        # if b"AtomicCompareExchange" in code_line:
        #   return True
        # elif b"f32" in code_line or b"f64" in code_line: # TODO: remove this (TEMP)
        #   return True
        # elif b"ctz" in code_line:
        #   return True
        # else:
        #   return False
        return False

    @override
    def filter_test_case(self, code):
        code_split = code.split(b"\n")
        func_start = [i for i, v in enumerate(code_split) if v.startswith(b"builder.addFunction(undefined")][0]
        func_end = [i for i, v in enumerate(code_split[func_start:]) if v == b"]);"][0] + func_start
        return not any([self.code_filter(i) for i in code_split[func_start:func_end]])

class JSCEngine(Engine):
    def __init__(self):
        super().__init__()

        # llint_exec = executor.LLIntExecutor(config.jsc_additional_configs) # no simd support
        bbq_exec = executor.BBQExecutor(config.jsc_additional_configs)
        omg_exec = executor.OMGExecutor(config.jsc_additional_configs)

        self.arch_list=config.jsc_arch_list
        # self.opt_range=range(3)
        # self.executor_list=[llint_exec, bbq_exec, omg_exec]
        self.opt_range=range(2)
        self.executor_list=[bbq_exec, omg_exec]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.JSWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithJSWrapper()
        else:
            assert False

        self.compare_list = list(itertools.product(config.jsc_arch_list, self.opt_range))

    def __str__(self):
        return "jsc"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        # if opt_level == 0:
        #     return "llint"
        # elif opt_level == 1:
        #     return "bbq"
        # else:
        #     return "omg"
        if opt_level == 0:
            return "bbq"
        else:
            return "omg"

    @override
    def str_to_opt_level(self, opt_str):
        # if opt_str == "llint":
        #     return 0
        # elif opt_str == "bbq":
        #     return 1
        # elif opt_str == "omg":
        #     return 2
        # else:
        #     assert False
        if opt_str == "bbq":
            return 0
        elif opt_str == "omg":
            return 1
        else:
            assert False

    @override
    def do_compare(self, workdir, feedbacks, seed):
        logger = logging.getLogger("fuzz")

        results = []
        results_raw = {}
        compile_success = True
        compile_success_archs = []
        is_crash_total = False

        for cl in self.compare_list:
            arch = cl[0]
            opt_level_str= self.opt_level_to_str(cl[1])
            code_path = os.path.join(workdir, ".cur.input")
            exec_result, exec_result2, returncode = self.executor_list[cl[1]].run(workdir, code_path, arch, feedbacks[arch], seed)

            is_crash = returncode != None and returncode < 0
            if opt_level_str != "llint": # simd crashes for llint
                is_crash_total |= is_crash

            results_raw[(arch, opt_level_str)] = (exec_result, exec_result2, is_crash)

            exec_result_post = self.postprocess_exec_result(exec_result)
            classified_result = self.executor_list[cl[1]].classify_result(exec_result_post)
            if classified_result == 'Success' and not is_crash:
                results.append(exec_result_post)
                compile_success_archs.append(cl)
                exec_result_post_hash = hashlib.md5(exec_result_post).hexdigest()
                logger.warning(f"[*] {arch}-{opt_level_str} exec result: {exec_result_post_hash}, {returncode}")
            elif classified_result == 'CompileError':
                compile_success = False
                logger.warning(f"[*] {arch}-{opt_level_str} compile failed: {returncode}")
            elif classified_result == 'RunError':
                compile_success = False
                logger.warning(f"[*] {arch}-{opt_level_str} run failed: {returncode}")
            elif classified_result == 'Timeout':
                logger.warning(f"[*] {arch}-{opt_level_str} timed out: {returncode}")
            elif is_crash:
                logger.warning(f"[*] {arch}-{opt_level_str} crashed: {returncode}")
            else:
                assert False

        is_success = len(set(results)) <= 1
        return compile_success, compile_success_archs, is_success, is_crash_total, results, results_raw


class WasmtimeEngine(Engine):
    def __init__(self):
        super().__init__()

        none_exec = executor.WasmtimeNoneExecutor(config.wasmtime_additional_configs)
        speed_exec = executor.WasmtimeSpeedExecutor(config.wasmtime_additional_configs)
        speedandsize_exec = executor.WasmtimeSpeedAndSizeExecutor(config.wasmtime_additional_configs)

        self.arch_list=config.wasmtime_arch_list
        self.opt_range=range(3) # None, Speed, SpeedAndSize
        self.executor_list=[none_exec, speed_exec, speedandsize_exec]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.RawWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithWrapper()
        else:
            assert False

        self.compare_list = list(itertools.product(config.wasmtime_arch_list, self.opt_range))

    def __str__(self):
        return "wasmtime"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        if opt_level == 0:
            return "none"
        elif opt_level == 1:
            return "speed"
        else:
            return "speedandsize"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "none":
            return 0
        elif opt_str == "speed":
            return 1
        elif opt_str == "speedandsize":
            return 2
        else:
            assert False


class WasmerEngine(Engine):
    def __init__(self):
        super().__init__()

        none_exec = executor.WasmerNoneExecutor(config.wasmer_additional_configs)
        speed_exec = executor.WasmerSpeedExecutor(config.wasmer_additional_configs)
        speedandsize_exec = executor.WasmerSpeedAndSizeExecutor(config.wasmer_additional_configs)
        llvmnone_exec = executor.WasmerLLVMNoneExecutor(config.wasmer_additional_configs)
        llvmless_exec = executor.WasmerLLVMLessExecutor(config.wasmer_additional_configs)
        llvmdef_exec = executor.WasmerLLVMDefExecutor(config.wasmer_additional_configs)
        llvmagg_exec = executor.WasmerLLVMAggExecutor(config.wasmer_additional_configs)
        # singlepass_exec = executor.WasmerSinglepassExecutor(config.wasmer_additional_configs)

        self.arch_list=config.wasmer_arch_list
        self.opt_range=range(7) # None, Speed, SpeedAndSize, LLVMNone, LLVMLess, LLVMDef, LLVMAgg, (Singlepass)
        self.executor_list=[
            none_exec, speed_exec, speedandsize_exec, 
            llvmnone_exec, llvmless_exec, llvmdef_exec, llvmagg_exec,
            # singlepass_exec
        ]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.RawWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithWrapper()
        else:
            assert False

        self.compare_list = []
        if "x64" in config.wasmer_arch_list:
            self.compare_list += list(itertools.product(["x64"], self.opt_range))
        x64_excluded = [arch for arch in config.wasmer_arch_list if arch != "x64"]
        self.compare_list += list(itertools.product(x64_excluded, range(3)))

    def __str__(self):
        return "wasmer"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        if opt_level == 0:
            return "none"
        elif opt_level == 1:
            return "speed"
        elif opt_level == 2:
            return "speedandsize"
        elif opt_level == 3:
            return "llvmnone"
        elif opt_level == 4:
            return "llvmless"
        elif opt_level == 5:
            return "llvmdef"
        elif opt_level == 6:
            return "llvmagg"
        else:
            return "singlepass"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "none":
            return 0
        elif opt_str == "speed":
            return 1
        elif opt_str == "speedandsize":
            return 2
        elif opt_str == "llvmnone":
            return 3
        elif opt_str == "llvmless":
            return 4
        elif opt_str == "llvmdef":
            return 5
        elif opt_str == "llvmagg":
            return 6
        elif opt_str == "singlepass":
            return 7
        else:
            assert False


class WasmedgeEngine(Engine):
    def __init__(self):
        super().__init__()

        vm_exec = executor.WasmedgeVMExecutor(config.wasmedge_additional_configs)
        O0_exec = executor.WasmedgeO0Executor(config.wasmedge_additional_configs)
        O1_exec = executor.WasmedgeO1Executor(config.wasmedge_additional_configs)
        O2_exec = executor.WasmedgeO2Executor(config.wasmedge_additional_configs)
        O3_exec = executor.WasmedgeO3Executor(config.wasmedge_additional_configs)
        Os_exec = executor.WasmedgeOsExecutor(config.wasmedge_additional_configs)
        Oz_exec = executor.WasmedgeOzExecutor(config.wasmedge_additional_configs)
        

        self.arch_list=config.wasmedge_arch_list
        self.opt_range=range(7) # VM, O0, O1, O2, O3, Os, Oz
        self.executor_list=[vm_exec, O0_exec, O1_exec, O2_exec, O3_exec, Os_exec, Oz_exec]
        if config.codegen_generator_option == 'stackgen':
            gen = stackgen.StackGenerator()
            self.generator = generator.RawWrapper(gen)
        elif config.codegen_generator_option == "wasm-smith":
            self.generator = generator.WasmSmithWrapper()
        else:
            assert False

        self.compare_list = []
        if "x64" in config.wasmedge_arch_list:
            self.compare_list += list(itertools.product(["x64"], self.opt_range))
        x64_excluded = [arch for arch in config.wasmedge_arch_list if arch != "x64"]
        self.compare_list += list(itertools.product(x64_excluded, range(3)))

    def __str__(self):
        return "wasmedge"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        if opt_level == 0:
            return "vm"
        elif opt_level == 1:
            return "O0"
        elif opt_level == 2:
            return "O1"
        elif opt_level == 3:
            return "O2"
        elif opt_level == 4:
            return "O3"
        elif opt_level == 5:
            return "Os"
        else:
            return "Oz"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "vm":
            return 0
        elif opt_str == "O0":
            return 1
        elif opt_str == "O1":
            return 2
        elif opt_str == "O2":
            return 3
        elif opt_str == "O3":
            return 4
        elif opt_str == "Os":
            return 5
        elif opt_str == "Oz":
            return 6
        else:
            assert False


# Evaluation
class WasmtimeXsmithEngine(Engine):
    def __init__(self):
        super().__init__()

        none_exec = executor.WasmtimeXsmithNoneExecutor(config.wasmtime_additional_configs)
        opt_exec = executor.WasmtimeXsmithOptExecutor(config.wasmtime_additional_configs)

        self.arch_list=config.wasmtime_xsmith_arch_list
        self.opt_range=range(2) # None, Opt
        self.executor_list=[none_exec, opt_exec]
        self.generator = generator.XsmithWrapper()

        self.compare_list = list(itertools.product(config.wasmtime_xsmith_arch_list, self.opt_range))

    def __str__(self):
        return "wasmtime-xsmith"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        if opt_level == 0:
            return "none"
        else:
            return "opt"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "none":
            return 0
        elif opt_str == "opt":
            return 1
        else:
            assert False

class WasmerXsmithEngine(Engine):
    def __init__(self):
        super().__init__()

        singlepass_exec = executor.WasmerXsmithSinglepassExecutor(config.wasmer_additional_configs)
        cranelift_exec = executor.WasmerXsmithCraneliftNoneExecutor(config.wasmer_additional_configs)
        cranelift_opt_exec = executor.WasmerXsmithCraneliftOptExecutor(config.wasmer_additional_configs)
        llvm_none_exec = executor.WasmerXsmithLLVMNoneExecutor(config.wasmer_additional_configs)
        llvm_agg_exec = executor.WasmerXsmithLLVMAggExecutor(config.wasmer_additional_configs)

        self.arch_list=config.wasmer_xsmith_arch_list
        self.opt_range=range(5) # singlepass, cranelift (none, speedandsize), llvm (none, agg)
        self.executor_list=[singlepass_exec, cranelift_exec, cranelift_opt_exec, llvm_none_exec, llvm_agg_exec]
        self.generator = generator.XsmithWrapper()

        self.compare_list = list(itertools.product(config.wasmer_xsmith_arch_list, self.opt_range))

    def __str__(self):
        return "wasmer-xsmith"

    @override
    def opt_level_to_str(self, opt_level):
        assert opt_level in self.opt_range
        if opt_level == 0:
            return "singlepass"
        elif opt_level == 1:
            return "cranelift-none"
        elif opt_level == 2:
            return "cranelift-opt"
        elif opt_level == 3:
            return "llvm-none"
        else:
            return "llvm-agg"

    @override
    def str_to_opt_level(self, opt_str):
        if opt_str == "singlepass":
            return 0
        elif opt_str == "cranelift-none":
            return 1
        elif opt_str == "cranelift-opt":
            return 2
        elif opt_str == "llvm-none":
            return 3
        elif opt_str == "llvm-agg":
            return 4
        else:
            assert False

class WasmedgeXsmithEngine(WasmedgeEngine):
    def __init__(self):
        super().__init__()

        self.arch_list=config.wasmedge_arch_list
        self.generator = generator.XsmithWrapper()

        self.compare_list = list(itertools.product(config.wasmedge_xsmith_arch_list, self.opt_range))


class V8XsmithEngine(V8Engine):
    def __init__(self):
        super().__init__()
        self.generator = generator.XsmithJSWrapper()


class SMXsmithEngine(SMEngine):
    def __init__(self):
        super().__init__()
        self.generator = generator.XsmithJSWrapper()


class JSCXsmithEngine(JSCEngine):
    def __init__(self):
        super().__init__()
        self.generator = generator.XsmithJSWrapper()
