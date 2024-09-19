import os
import multiprocessing
import argparse

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = source_dir

# arg parsing
parser = argparse.ArgumentParser(description='Please put eval args')
parser.add_argument(
    '--eval-name', 
    dest='eval_name', 
    type=str, 
    default='unused', 
    help='name for evaluation'
)
parser.add_argument(
    '--num-processes', 
    metavar='-n', 
    dest='num_processes', 
    type=int, 
    default=multiprocessing.cpu_count(), 
    help='number of processes'
)
parser.add_argument(
    '--extractor-opt', 
    dest='extractor_opt', 
    type=str, 
    default='all', 
    help='extractor option (all, opt, lower, typing, optlower, opttyping, lowertyping)'
)
parser.add_argument(
    '--generator-opt', 
    dest='generator_opt', 
    type=str,
    default='stackgen', 
    help='generator option (stackgen, wasm-smith)'
)
parser.add_argument(
    '--generator-seed-random', 
    dest='generator_seed_random', 
    type=bool,
    default=True, 
    help='generator seed option (use random)'
)
parser.add_argument(
    '--engine-name', 
    dest='engine_name', 
    type=str,
    default='wasmtime',
    help='target engine name (wasmtime, v8, sm, jsc, wasmer, wasmedge)'
)
parser.add_argument(
    '--archs',
    dest='archs_override',
    type=str,
    default='',
    help='target arch list (x64, arm64, loong64, ia32, arm, riscv32, riscv64, mips64el, s390x, ppc64) - comma separation'
)
parser.add_argument(
    '--save-crashes', 
    dest='save_crashes', 
    action=argparse.BooleanOptionalAction,
    type=bool,
    default=False, 
    help='save crashes'
)
parser.add_argument(
    '--canonicalize-nans',
    dest='canonicalize_nans',
    action=argparse.BooleanOptionalAction,
    type=bool,
    default=True,
    help='canonicalize nans for wasm-smith'
)
args = parser.parse_args()

# v8 configs
v8_path = "/fuzz/targets/v8/v8"
v8_binary_path = os.path.join(v8_path, "out")
v8_arch_list = [
    "x64",
    "arm64",
    "loong64",
    "ia32",
    "arm",
    "riscv32",
    "riscv64",
    "mips64el",
    "s390x",
    "ppc64"
] if len(args.archs_override) == 0 else args.archs_override.split(',')
# v8_additional_configs = ["--experimental-wasm-gc"]
v8_additional_configs = []


# SM configs
sm_path = "/fuzz/targets/spidermonkey"
sm_binary_path = os.path.join(sm_path, "builds")
sm_arch_list = [
    "x64", "arm64", "ia32", "arm", "loong64" # mips fails to compile
] if len(args.archs_override) == 0 else args.archs_override.split(',')
sm_build_type = "coverage" # build type: ("release", "coverage", "debug")
sm_additional_configs = []

# JSC configs
jsc_path = "/fuzz/targets/jsc"
jsc_binary_path = os.path.join(jsc_path, "builds")
jsc_arch_list = ["x64", "arm64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
jsc_additional_configs = []
br2_host_dir = "/home/candymate/tools/toolchains/aarch64/buildroot-2023.02.1" # buildroot

# Wasmtime configs
wasmtime_path = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmtime", "wasmtime-wrapper")
wasmtime_binary_path = os.path.join(wasmtime_path, "target")
wasmtime_arch_list = ["x64", "arm64", "riscv64", "s390x"] if len(args.archs_override) == 0 else args.archs_override.split(',')
wasmtime_additional_configs = []

# Wasmer configs
wasmer_path = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmer", "wasmer-wrapper")
wasmer_binary_path = os.path.join(wasmer_path, "target")
wasmer_arch_list = ["x64", "arm64", "riscv64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
wasmer_additional_configs = []

# Wasmedge configs
wasmedge_path = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmedge", "wasmedge-wrapper")
wasmedge_binary_path = os.path.join(wasmedge_path, "target")
wasmedge_arch_list = ["x64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
wasmedge_additional_configs = []

# codegen configs
codegen_seed_len = 4 # length condition of seed accepted by the generator
codegen_is_random = args.generator_seed_random # is random generation or consumption generation
codegen_seed_enum_start_from = 0 # when enumerative
codegen_extractor_option = args.extractor_opt # extractor option (all, opt, lower, typing, optlower, opttyping, lowertyping)
codegen_generator_option = args.generator_opt # generator option (stackgen, wasm-smith)

codegen_memory_max = 1 # in num of pages (for each, 64kiB)
codegen_table_size = 65536
codegen_blacklist_types = [] # e.g., ['funcref', 'externref']

# codegen interesting values
codegen_interesting_i32 = (0, 1, 2, 31, 32, 42, 63, 64, 0xff, 0x1000-1, 0x1000, 0x1000+1, 0xffff, 0x7fffffff,
                           -1, -2, -31, -32, -42, -63, -64, -0xff, -(0x1000-1), -0x1000, -(0x1000+1), -0xffff, -0x7fffffff, -0x80000000)
codegen_interesting_i64 = codegen_interesting_i32 + (0x80000000, 0xffffffff, -0xffffffff, -(1 << 63), (1 << 63)-1, -((1 << 63)-1))
codegen_interesting_f32 = (+0.0, -0.0, 1.0, -1.0, 4096.0, -4096.0,
                           float((1 << 31)), float((1 << 31)-1), float((1 << 63)), float((1 << 63)-1), 
                           float((1 << 32)), float((1 << 32)-1), float((1 << 64)), float((1 << 64)-1), 
                           -float((1 << 31)), -float((1 << 31)-1), -float((1 << 63)), -float((1 << 63)-1),
                           -float((1 << 32)), -float((1 << 32)-1), -float((1 << 64)), -float((1 << 64)-1),  
                           float('inf'), float('-inf'), float('NaN'))
codegen_interesting_f64 = codegen_interesting_f32
codegen_interesting_v128 = ( # work as little endian
    0x00000000000000000000000000000000,
    0x40404040404040404040404040404040,
    0x80808080808080808080808080808080,
    0xcccccccccccccccccccccccccccccccc,
    0xffffffffffffffffffffffffffffffff,
    0xfffefdfccccdcecf807f7e7d00010203,
    0x00010203cccdcecf807f7e7dfffefdfc,
    0x01010101010101010101010101010101,
    0xfefefefefefefefefefefefefefefefe,
)
codegen_interesting_laneidx16 = ( # work as big endian
    0x00020406080a0c0e10121416181a1c1e,
    0x01030507090b0d0f11131517191b1d1f,
    0x0001040508090c0d1011141518191c1d,
    0x020306070a0b0e0f121316171a1b1e1f,
    0x0001020308090a0b1011121318191a1b,
    0x040506070c0d0e0f141516171c1d1e1f,
    0x00010203040506071011121314151617,
    0x08090a0b0c0d0e0f18191a1b1c1d1e1f,
    0x00100111021203130414051506160717,
    0x081809190a1a0b1b0c1c0d1d0e1e0f1f,
    0x00011011020312130405141506071617,
    0x080918190a0b1a1b0c0d1c1d0e0f1e1f,
    0x00010203101112130405060714151617,
    0x08090a0b18191a1b0c0d0e0f1c1d1e1f,
    0x001002120414061608180a1a0c1c0e1e,
    0x011103130515071709190b1b0d1d0f1f,
    0x0001101104051415080918190c0d1c1d,
    0x02031213060716170a0b1a1b0e0f1e1f,
    0x000102031011121308090a0b18191a1b,
    0x04050607141516170c0d0e0f1c1d1e1f,
    0x010003020504070609080b0a0d0c0f0e,
    0x03020100070605040b0a09080f0e0d0c,
    0x02030001060704050a0b08090e0f0c0d,
    0x07060504030201000f0e0d0c0b0a0908,
    0x06070405020300010e0f0c0d0a0b0809,
    0x04050607000102030c0d0e0f08090a0b,
)

# codegen probability configs
## common
codegen_prob_const_use_interesting = 0.9 # how often interesting constants are used
codegen_prob_perturb = 0.05 # how often to perturb operands
codegen_prob_use_typing = 0.8 # how often to use typing rules
codegen_prob_memarg_inbounds = 0.99 # how often memarg is in bounds

## stackgen
codegen_stackgen_prob_multiret = 0.2 # probability for multiple return
codegen_stackgen_prob_struct_gen = 0.1 # probability for structure generation
codegen_stackgen_maximum_depth = 5 # maximum structure depth
codegen_stackgen_prob_call = 0.1 # probability for calls
codegen_stackgen_prob_call_indirect = 0.001 # probability for call_indirect when generating call (exec will always fail)
codegen_stackgen_prob_unreachable = 0.0001 # probability for unreachable (this is not desired)
codegen_stackgen_prob_struct_exit = 0.1 # probability for structure exit
codegen_stackgen_prob_struct_ret = 0.5 # probability of using the ret on stack
codegen_stackgen_prob_struct_skipelse = 0.8 # probability of skipping the else
codegen_stackgen_prob_stack_pop = 0.9 # probability of using the type in the stack
codegen_stackgen_prob_reuse_local = 0.2 # probability of reusing the locals
codegen_stackgen_prob_reuse_global = 0.5 # probability of reusing the globals
codegen_stackgen_prob_reuse_func = 0.9 # probability of reusing the functions
codegen_stackgen_prob_br = 0.05 # probability of br
codegen_stackgen_prob_br_if = 0.05 # probability of br_if
codegen_stackgen_prob_br_table_conti = 0.5 # probability for multiple labels
codegen_stackgen_prob_argconst = 0.25 # probability of removing a type with const or arg
codegen_stackgen_prob_constgen = 0.5 # probability of constant when terminating recursion
codegen_stackgen_prob_var_gen = 0.05 # probability of global or local when terminating recursion
codegen_stackgen_prob_globalgen = 0.2 # probability of global when global or local is generated

# run configs
run_interesting_i32 = codegen_interesting_i32
run_interesting_i64 = codegen_interesting_i64
run_interesting_f32 = codegen_interesting_f32
run_interesting_f64 = codegen_interesting_f64
run_interesting_v128 = codegen_interesting_v128
executor_timeout_sec = 3
extractor_timeout_sec = 600

# logging configs
logging_level = 'WARNING'
log_file_limit = 5*1024*1024 # 5MB
log_backup_cnt = 2

# main configs
engine_name = args.engine_name
catch_compile_error = True
save_crashes = args.save_crashes

## multiprocessing
num_processes = args.num_processes
main_print_frequency = 100 # print result for every n cases
max_tasks_per_child = 1000
multiprocessing_chunksize = 10

## directories
rootdir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
crash_save_rootdir = os.path.join(rootdir, "crashes")
interesting_save_rootdir = os.path.join(rootdir, "interesting")
workdir = os.path.join(rootdir, "run")
main_stop_after_crash = False

# eval configs
coverage_dir = os.path.join(rootdir, "coverage", args.eval_name)

## targets
xsmith_base_path = "/fuzz/targets/eval/xsmith"
xsmith_codegen_path = os.path.join(xsmith_base_path, "wasmlike")
xsmith_wasmtime_path = os.path.join(xsmith_base_path, "wasmtime", "load-wasmtime")
wasmtime_xsmith_arch_list = ["x64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
xsmith_wasmer_path = os.path.join(xsmith_base_path, "wasmer", "load-wasmer")
wasmer_xsmith_arch_list = ["x64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
wasmedge_xsmith_arch_list = ["x64"] if len(args.archs_override) == 0 else args.archs_override.split(',')
wasm_smith_seed_len = 4096
wasm_smith_canon_nans = args.canonicalize_nans
