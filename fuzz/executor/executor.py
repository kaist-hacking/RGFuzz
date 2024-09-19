from overrides import override
import os
import sys
import subprocess
import logging
import shutil

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir
sys.path[0] = root_dir

import config

class Executor():
    def __init__(self, concrete_exec_addflag=[]):
        self.concrete_exec_addflag = concrete_exec_addflag

    def run(self, workdir, code_path, arch, feedbacks, seed):
        return b""

    # TODO: crash oracle
    def classify_result(self, result):
        if result == b"Timeout":
            return 'Timeout'
        elif b"CompileError" in result or b"TypeError" in result:
            return "CompileError"
        elif b"No such file or directory" in result:
            return "RunError"
        elif result == b"":
            return "RunError"
        else:
            return "Success"


class V8Executor(Executor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag)

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        progname = os.path.join(arch + ".release", "d8")
        progname_abs = os.path.join(config.v8_binary_path, progname)
        assert os.path.exists(progname_abs) and os.path.isfile(progname_abs)

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "v8", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          [progname_abs, code_path_abs]+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()
        return result, result2, subp.returncode


class LiftoffExecutor(V8Executor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--liftoff"])


class TurboFanExecutor(V8Executor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--no-liftoff"])


class SMExecutor(Executor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag)

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        progname = os.path.join(arch + "_"+config.sm_build_type, "dist", "bin", "js")
        progname_abs = os.path.join(config.sm_binary_path, progname)
        assert os.path.exists(progname_abs) and os.path.isfile(progname_abs)

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "sm", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          [progname_abs, code_path_abs]+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        returncode = subp.returncode
        if config.sm_build_type == "coverage": # since coverage build crashes on ud2 (unreachable)
            returncode = 0 # Cons: cannot catch crash bugs while in evaluation
        return result, result2, returncode


class BaselineExecutor(SMExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--wasm-compiler=baseline"])


class IonExecutor(SMExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--wasm-compiler=ion"])


class JSCExecutor(Executor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--useConcurrentJIT=false"])

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        progname = os.path.join(arch, "bin", "jsc")
        progname_abs = os.path.join(config.jsc_binary_path, progname)
        assert os.path.exists(progname_abs) and os.path.isfile(progname_abs)

        exec_params = []
        if arch == "x64":
            exec_params = [progname_abs, code_path_abs]
        else:
            arch_path = arch if arch != "arm64" else "aarch64"
            qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}")
            qemu_path = os.path.join(config.br2_host_dir, f"{arch_path}-buildroot-linux-gnu", "sysroot")
            qemu_lib_path = os.path.join(qemu_path, "lib")

            exec_params.append(qemu_bin_path)
            exec_params.append("-L")
            exec_params.append(qemu_path)
            exec_params.append("-E")
            exec_params.append("LD_LIBRARY_PATH="+qemu_lib_path)
            exec_params.append(progname_abs)
            exec_params.append(code_path_abs)

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "jsc", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()
        return result, result2, subp.returncode


class LLIntExecutor(JSCExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--useWasmLLInt=true", "--useBBQJIT=false", "--useOMGJIT=false"])


class BBQExecutor(JSCExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(concrete_exec_addflag+["--useWasmLLInt=true", "--useBBQJIT=true", "--useOMGJIT=false"])


class OMGExecutor(JSCExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        new_concrete_exec_addflag = ["--useWasmLLInt=true", "--useBBQJIT=false", "--useOMGJIT=true"]
        super().__init__(concrete_exec_addflag+new_concrete_exec_addflag)


class WasmtimeExecutor(Executor):
    arch_target_dict = {
      "x64": "",
      "arm64": "aarch64-unknown-linux-gnu",
      "riscv64": "riscv64gc-unknown-linux-gnu",
      "s390x": "s390x-unknown-linux-gnu",
    }

    qemu_path_dict = {
      "x64": "",
      "arm64": "aarch64-linux-gnu",
      "riscv64": "riscv64-linux-gnu",
      "s390x": "s390x-linux-gnu"
    }

    def __init__(self, concrete_exec_addflag=[], opt_level=0):
        super().__init__(concrete_exec_addflag)
        self.opt_level = opt_level

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        exec_params = []
        if arch == "x64":
            target_path = os.path.join(config.wasmtime_binary_path, "release", "wasmtime-wrapper")
        else:
            arch_path = arch if arch != "arm64" else "aarch64"

            if arch == "riscv64":
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}") # from riscv-gnu-toolchain
            else:
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}")
            qemu_path = os.path.join("/usr", self.qemu_path_dict[arch])
            qemu_lib_path = os.path.join(qemu_path, "lib")

            target_name = self.arch_target_dict[arch]
            target_path = os.path.join(config.wasmtime_binary_path, target_name, "release", "wasmtime-wrapper")
            exec_params.append(qemu_bin_path)
            if arch == "riscv64":
                exec_params.append("-cpu")
                exec_params.append("rv64,v=true,vlen=128,vext_spec=v1.0,zba=true,zbb=true,zbs=true,zbc=true,zbkb=true,zcb=true,zicond=true")
            exec_params.append("-L")
            exec_params.append(qemu_path)
            exec_params.append("-E")
            exec_params.append("LD_LIBRARY_PATH="+qemu_lib_path)
            exec_params.append("-E")
            exec_params.append("WASMTIME_TEST_NO_HOG_MEMORY=1")

        exec_params.append(target_path)
        exec_params.append(code_path_abs)
        exec_params.append(str(self.opt_level))
        exec_params.append(str(seed & ((1 << 64) - 1)))

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "wasmtime", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
            result = result
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        return result, result2, subp.returncode


class WasmtimeNoneExecutor(WasmtimeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=0)


class WasmtimeSpeedExecutor(WasmtimeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=1)


class WasmtimeSpeedAndSizeExecutor(WasmtimeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=2)


class WasmerExecutor(Executor):
    arch_target_dict = {
      "x64": "",
      "arm64": "aarch64-unknown-linux-gnu",
      "riscv64": "riscv64gc-unknown-linux-gnu",
    }

    qemu_path_dict = {
      "x64": "",
      "arm64": "aarch64-linux-gnu",
      "riscv64": "riscv64-linux-gnu",
    }

    def __init__(self, concrete_exec_addflag=[], opt_level=0):
        super().__init__(concrete_exec_addflag)
        self.opt_level = opt_level

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        exec_params = []
        if arch == "x64":
            target_path = os.path.join(config.wasmer_binary_path, "release", "wasmer-wrapper")
        else:
            arch_path = arch if arch != "arm64" else "aarch64"

            if arch == "riscv64":
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}") # from riscv-gnu-toolchain
            else:
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}")
            qemu_path = os.path.join("/usr", self.qemu_path_dict[arch])
            qemu_lib_path = os.path.join(qemu_path, "lib")

            target_name = self.arch_target_dict[arch]
            target_path = os.path.join(config.wasmer_binary_path, target_name, "release", "wasmer-wrapper")
            exec_params.append(qemu_bin_path)
            if arch == "riscv64":
                exec_params.append("-cpu")
                exec_params.append("rv64,v=true,vlen=128,vext_spec=v1.0,zba=true,zbb=true,zbs=true,zbc=true,zbkb=true,zcb=true,zicond=true")
            exec_params.append("-L")
            exec_params.append(qemu_path)
            exec_params.append("-E")
            exec_params.append("LD_LIBRARY_PATH="+qemu_lib_path)
            exec_params.append("-E")
            exec_params.append("WASMTIME_TEST_NO_HOG_MEMORY=1")

        exec_params.append(target_path)
        exec_params.append(code_path_abs)
        exec_params.append(str(self.opt_level))
        exec_params.append(str(seed & ((1 << 64) - 1)))

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "wasmer", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
            result = result
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        return result, result2, subp.returncode


class WasmerNoneExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=0)


class WasmerSpeedExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=1)


class WasmerSpeedAndSizeExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=2)


class WasmerLLVMNoneExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=3)


class WasmerLLVMLessExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=4)


class WasmerLLVMDefExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=5)


class WasmerLLVMAggExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=6)


class WasmerSinglepassExecutor(WasmerExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=7)


class WasmedgeExecutor(Executor):
    def __init__(self, concrete_exec_addflag=[], opt_level=0):
        super().__init__(concrete_exec_addflag)
        self.opt_level = opt_level

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        # change file extension
        shutil.copyfile(code_path_abs, code_path_abs + ".wasm")
        code_path_abs += ".wasm"

        exec_params = []
        if arch == "x64":
            target_path = os.path.join(config.wasmedge_binary_path, "release", "wasmedge-wrapper")
        else:
            assert False

        exec_params.append(target_path)
        exec_params.append(code_path_abs)
        exec_params.append(workdir) # out_dir
        exec_params.append(str(self.opt_level))
        exec_params.append(str(seed & ((1 << 64) - 1)))

        env = {
            "LD_LIBRARY_PATH": os.path.join(os.path.dirname(config.wasmedge_path), "WasmEdge", "build", "lib", "api"),
            "LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "wasmedge", arch, f"{arch}_%{config.num_processes}m.profraw")
        }

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
            result = result
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        return result, result2, subp.returncode


class WasmedgeVMExecutor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=0)


class WasmedgeO0Executor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=1)


class WasmedgeO1Executor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=2)


class WasmedgeO2Executor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=3)


class WasmedgeO3Executor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=4)


class WasmedgeOsExecutor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=5)


class WasmedgeOzExecutor(WasmedgeExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=6)


# Evaluation
class WasmtimeXsmithExecutor(Executor):
    arch_target_dict = {
      "x64": "",
      "arm64": "aarch64-unknown-linux-gnu",
      "riscv64": "riscv64gc-unknown-linux-gnu",
      "s390x": "s390x-unknown-linux-gnu",
    }

    qemu_path_dict = {
      "x64": "",
      "arm64": "aarch64-linux-gnu",
      "riscv64": "riscv64-linux-gnu",
      "s390x": "s390x-linux-gnu"
    }

    def __init__(self, concrete_exec_addflag=[], opt_level=0):
        super().__init__(concrete_exec_addflag)
        self.opt_level = opt_level

    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        exec_params = []
        if arch == "x64":
            target_path = os.path.join(config.xsmith_wasmtime_path, "target", "release", "load-wasmtime")
        else:
            arch_path = arch if arch != "arm64" else "aarch64"

            if arch == "riscv64":
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}") # from riscv-gnu-toolchain
            else:
                qemu_bin_path = os.path.join("/usr/local/bin", f"qemu-{arch_path}")
            qemu_path = os.path.join("/usr", self.qemu_path_dict[arch])
            qemu_lib_path = os.path.join(qemu_path, "lib")

            target_name = self.arch_target_dict[arch]
            target_path = os.path.join(config.xsmith_wasmtime_path, "target", target_name, "release", "load-wasmtime")
            exec_params.append(qemu_bin_path)
            if arch == "riscv64":
                exec_params.append("-cpu")
                exec_params.append("rv64,v=true,vlen=128,vext_spec=v1.0,zba=true,zbb=true,zbs=true,zbc=true,zbkb=true,zcb=true,zicond=true")
            exec_params.append("-L")
            exec_params.append(qemu_path)
            exec_params.append("-E")
            exec_params.append("LD_LIBRARY_PATH="+qemu_lib_path)
            exec_params.append("-E")
            exec_params.append("WASMTIME_TEST_NO_HOG_MEMORY=1")

        exec_params.append(target_path)
        if self.opt_level == 1:
            exec_params.append("--optimize=true")
        exec_params.append(code_path_abs)

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "wasmtime-xsmith", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
            result = result
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        return result, result2, subp.returncode


class WasmtimeXsmithNoneExecutor(WasmtimeXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=0)


class WasmtimeXsmithOptExecutor(WasmtimeXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=1)


class WasmerXsmithExecutor(Executor):
    def __init__(self, concrete_exec_addflag=[], opt_level=0):
        super().__init__(concrete_exec_addflag)
        self.opt_level = opt_level
    
    @override
    def run(self, workdir, code_path, arch, feedbacks, seed):
        code_path_abs = code_path
        if not os.path.isabs(code_path):
            code_path_abs = os.path.join(root_dir, code_path)
        assert os.path.exists(code_path_abs) and os.path.isfile(code_path_abs)

        exec_params = []
        if arch == "x64":
            target_path = os.path.join(config.xsmith_wasmer_path, "target", "release", "load-wasmer")
        else:
            assert False

        exec_params.append(target_path)
        if self.opt_level == 0:
            exec_params.append("--compiler=singlepass")
        elif self.opt_level == 1:
            exec_params.append("--compiler=cranelift")
            exec_params.append("--optimize=false")
        elif self.opt_level == 2:
            exec_params.append("--compiler=cranelift")
            exec_params.append("--optimize=true")
        elif self.opt_level == 3:
            exec_params.append("--compiler=llvm")
            exec_params.append("--optimize=false")
        elif self.opt_level == 4:
            exec_params.append("--compiler=llvm")
            exec_params.append("--optimize=true")
        exec_params.append(code_path_abs)

        env = {"LLVM_PROFILE_FILE": os.path.join(config.coverage_dir, "wasmer-xsmith", arch, f"{arch}_%{config.num_processes}m.profraw")}

        subp = subprocess.Popen(
          exec_params+self.concrete_exec_addflag, env=env,
          shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        result2 = b""
        try:
            result, result2 = subp.communicate(timeout=config.executor_timeout_sec)
            result = result
        except subprocess.TimeoutExpired:
            result = b"Timeout"
        subp.kill()

        return result, result2, subp.returncode


class WasmerXsmithSinglepassExecutor(WasmerXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=0)


class WasmerXsmithCraneliftNoneExecutor(WasmerXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=1)


class WasmerXsmithCraneliftOptExecutor(WasmerXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=2)


class WasmerXsmithLLVMNoneExecutor(WasmerXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=3)


class WasmerXsmithLLVMAggExecutor(WasmerXsmithExecutor):
    def __init__(self, concrete_exec_addflag=[]):
        super().__init__(opt_level=4)
