#!/usr/bin/env python3

# This script/reducer is designed for wasmlike at the cost of being less general for other fuzzers.
# For example: assuming that the last step in each test in the config file is the one that produces the
# answer, and doesn't require any previously saved tokens like $fuzzer-output

import argparse
import subprocess
import shlex
import time
import yaml
import sys
import os
import stat
import re

if not sys.version_info >= (3, 7):
    print("Error! `python3` must be at least version 3.7")
    exit(1)


def run_subshell(command):  # provides the command after running the subshell
    subshell_pattern = re.compile(r'\$\((.*)\)')
    subshell = re.search(subshell_pattern, command)
    if subshell:
        # Extract the command and run it
        subshell_command = subshell.expand(r'\g<1>').strip()
        subshell_result = subprocess.run(subshell_command, capture_output=True, shell=True)

        # _, subshell_out, _ = bash['-c', subshell_command].run(cwd=test_subdir, retcode=None)
        # Replace the subshell command in the tool with the result
        updated_command = re.sub(subshell_pattern, subshell_result.stdout.strip().decode(), command)
        return updated_command

    return command


parser = argparse.ArgumentParser(description='Given a config file, xsmith generation options, and two \
                                              configuration names, this script will perform reducing \
                                              operations on the generated program.')

parser.add_argument("--config", "-c", help="The path of the configuration file containing systems under test",
                    type=str, required=True)
parser.add_argument("--runtime-a", "-a", help="The quoted name of the runtime in the configuration file",
                    type=str, required=True)
parser.add_argument("--runtime-b", "-b", help="The quoted name of the other runtime in the configuration file",
                    type=str, required=True)
parser.add_argument("--runtime-c", help="The quoted name of another runtime in the configuration file (optional)",
                    type=str, required=False, default=None)
parser.add_argument("--wasmlike", "-w", help="Optional. The location of wasmlike if it \
                                              is not in the default path for emulab experiments",
                    type=str, default='/local/work/webassembly-sandbox/wasmlike/wasmlike.rkt')
parser.add_argument("--reducer", "-r", help="Optional. The location of `wasm-tools shrink` if it \
                                                 is not in the default path for emulab experiments",
                    type=str, default='wasm-tools shrink')
# Exclusive group for options or program
program_group = parser.add_mutually_exclusive_group(required=True)
program_group.add_argument("--xsmith-options", "-x", help="The xsmith options to generate a program. Meant to be used \
                                                           to verify that two outputs differ",
                           type=str)
program_group.add_argument("--program", "-p", help="The path to an already generated .wasm program. Meant to be used \
                                                    when coupled with --reduce",
                           type=str)
# Exclusive group for verification or reduction
mode_group = parser.add_mutually_exclusive_group(required=True)
mode_group.add_argument("--verify", help="Runs the program on both runtimes to time and verify the outputs. \
                                          Does not reduce",
                        action="store_true")
mode_group.add_argument("--reduce", help="Reduces the program. Make sure to run in verification mode first",
                        action="store_true")
mode_group.add_argument("--compare", help="Reducing predicate: outputs 'equal' or 'not equal'",
                        action="store_true")
parser.add_argument("--time", help="Print execution time along with the program output. Do not use with reduce mode",
                    action='store_true')
parser.add_argument("--timeout", help="Set the timeout in seconds. Default is '10'. \
                                       Long timeouts will make reducing very slow.",
                    type=int, default=10)
parser.add_argument("--non-empty", help="Treat empty stdout (crashes) as not interesting for reducing",
                    action='store_true')
# debug mode
parser.add_argument("--debug-args", help="Print the parsed arguments and exit", action='store_true')

args = parser.parse_args()

program_group_choice = "xsmith-options"
program_group_data = args.xsmith_options
if args.program:
    program_group_choice = "program"
    program_group_data = args.program


mode_group_choice = "verify"
if args.reduce:
    mode_group_choice = "reduce"

if args.debug_args:
    output = f"\nParsed arguments \n-------------------------------\n" \
             f"Config [{args.config}]\n" \
             f"Runtime A [{args.runtime_a}]\n" \
             f"Runtime B [{args.runtime_b}]\n" \
             f"Runtime C [{args.runtime_c}]\n" \
             f"Time [{args.time}]\n" \
             f"Timeout [{args.timeout}]\n" \
             f"wasmlike [{args.wasmlike}]\n" \
             f"`wasm-tools shrink` [{args.reducer}]\n" \
             f"Program group [{program_group_choice}] [{program_group_data}]\n" \
             f"Mode group [{mode_group_choice}]\n"

    print(output)
    exit(0)

# Find the system under test by name in the config
with open(args.config, 'r') as f:
    config = yaml.load(f, Loader=yaml.BaseLoader)
systems_under_test = config['tests']

runtime_a_cmd = None
runtime_b_cmd = None
runtime_c_cmd = None

try:
    runtime_a_commands = next(system['commands'] for system in systems_under_test if system['name'] == args.runtime_a)
    runtime_a_cmd = list(runtime_a_commands[-1].values())[0].strip()  # Get the last command
    runtime_a_cmd = runtime_a_cmd.replace("$$", "$")
    runtime_a_cmd = run_subshell(runtime_a_cmd)
except StopIteration:
    print(f"System under test: {args.runtime_a} not found in configuration file")
    exit(1)

try:
    runtime_b_commands = next(system['commands'] for system in systems_under_test if system['name'] == args.runtime_b)
    runtime_b_cmd = list(runtime_b_commands[-1].values())[0].strip()
    runtime_b_cmd = runtime_b_cmd.replace("$$", "$")
    runtime_b_cmd = run_subshell(runtime_b_cmd)
except StopIteration:
    print(f"System under test: {args.runtime_b} not found in configuration file")
    exit(1)

if args.runtime_c:
    try:
        runtime_c_commands = next(system['commands'] for system in systems_under_test if system['name'] == args.runtime_c)
        runtime_c_cmd = list(runtime_c_commands[-1].values())[0].strip()
        runtime_c_cmd = runtime_c_cmd.replace("$$", "$")
        runtime_c_cmd = run_subshell(runtime_c_cmd)
    except StopIteration:
        print(f"System under test: {args.runtime_c} not found in configuration file")
        exit(1)


# compile and run wasmlike
if args.xsmith_options:
    raco_make_cmd = shlex.split(f"raco make {args.wasmlike}")
    subprocess.run(raco_make_cmd)

# generate the program if needed
if args.xsmith_options:
    generate_cmd = shlex.split(f"racket {args.wasmlike} {args.xsmith_options} -o generated_wasm.wat")
    subprocess.run(generate_cmd)
    convert_cmd = shlex.split(f"wat2wasm generated_wasm.wat -o test_wasm.wasm")
    subprocess.run(convert_cmd)
elif args.program:
    # make sure the program is named correctly.
    if args.program != "test_wasm.wasm":
        subprocess.run(shlex.split(f"cp {args.program} test_wasm.wasm"))

# Save the original to refer back to
subprocess.run(shlex.split(f"cp test_wasm.wasm orig_wasm.wasm"))

# Run the two runtimes with a timeout and collect the output
start_time = time.time()
runtime_a_result = subprocess.run(shlex.split(f"timeout {args.timeout}s {runtime_a_cmd}"), capture_output=True)
elapsed_a = time.time() - start_time

start_time = time.time()
runtime_b_result = subprocess.run(shlex.split(f"timeout {args.timeout}s {runtime_b_cmd}"), capture_output=True)
elapsed_b = time.time() - start_time

runtime_c_result = None
elapsed_c = None
if args.runtime_c:
    start_time = time.time()
    runtime_c_result = subprocess.run(shlex.split(f"timeout {args.timeout}s {runtime_c_cmd}"), capture_output=True)
    elapsed_c = time.time() - start_time

if args.verify or args.reduce:
    if args.reduce:
        print("Verify that the following outputs are correct:")

    if runtime_a_result.returncode == 127:
        print(f"A: {runtime_a_result.stderr.decode()}")
    if runtime_b_result.returncode == 127:
        print(f"B: {runtime_b_result.stderr.decode()}")
    if runtime_c_result and runtime_c_result.returncode == 127:
        print(f"C: {runtime_c_result.stderr.decode()}")

    # if runtime_a_result.returncode == 127 or runtime_b_result.returncode == 127:
    #     exit(1)

    if runtime_a_result.returncode == 124:
        print(f"A: ({args.timeout}s) Timeout")
    else:
        if args.time:
            print(f"A: ({int(elapsed_a)}s) {runtime_a_result.stdout.decode()}")
        else:
            print(f"A: {runtime_a_result.stdout.decode()}")

    if runtime_b_result.returncode == 124:
        print(f"B: ({args.timeout}s) Timeout")
    else:
        if args.time:
            print(f"B: ({int(elapsed_b)}s) {runtime_b_result.stdout.decode()}")
        else:
            print(f"B: {runtime_b_result.stdout.decode()}")

    if runtime_c_result and runtime_c_result.returncode == 124:
        print(f"C: ({args.timeout}s) Timeout")
    elif runtime_c_result:
        if args.time:
            print(f"C: ({int(elapsed_c)}s) {runtime_c_result.stdout.decode()}")
        else:
            print(f"C: {runtime_c_result.stdout.decode()}")

if args.compare:
    # wasm-tools shrink usage: The predicate script must exit with a zero status code if
    # the Wasm file is interesting and a non-zero status code otherwise.
    if runtime_a_result.returncode == 1 \
            or runtime_b_result.returncode == 1 \
            or (runtime_c_result and runtime_c_result.returncode == 1):
        print("error with reduced test file")  # Most common reason is stripping out the export of '_main'
        print(runtime_a_result.stdout)
        print(runtime_a_result.stderr)
        print(runtime_b_result.stdout)
        print(runtime_b_result.stderr)
        if runtime_c_result:
            print(runtime_c_result.stderr)
            print(runtime_c_result.stdout)
        exit(4)
    elif runtime_a_result.returncode == 124 \
            or runtime_b_result.returncode == 124 \
            or (runtime_c_result and runtime_c_result.returncode == 124):
        if runtime_a_result.returncode == 124:
            print("Timeout on A")
        if runtime_b_result.returncode == 124:
            print("Timeout on B")
        if runtime_c_result and runtime_c_result.returncode == 124:
            print("Timeout on C")
        exit(5)

    # Non zero return codes aren't interesting. It means there's been a crash or something went wrong
    if runtime_a_result.returncode != 0 \
            or runtime_b_result.returncode != 0 \
            or (runtime_c_result and runtime_c_result.returncode != 0):
        exit(6)

    if args.non_empty:
        if runtime_a_result.stdout.strip() == b'' \
                or runtime_b_result.stdout.strip() == b'' \
                or runtime_c_result and runtime_c_result.stdout.strip() == b'':
            exit(7)

    elif runtime_c_result is None \
            and runtime_a_result.stdout == runtime_b_result.stdout:
        exit(8)

    elif runtime_c_result \
            and (runtime_a_result.stdout == runtime_b_result.stdout
                 or runtime_a_result.stdout == runtime_c_result.stdout
                 or runtime_b_result.stdout == runtime_c_result.stdout):
        exit(7)
    elif subprocess.run(shlex.split(f"./predicate.py test_wasm.wasm")).returncode == 1:
        exit(10) # program is not interesting if it computes NaN values
    else:
        # print("not equal")
        exit(0)

# if args.verify:
#     with open("output.txt", mode='r') as f:
#         print(f.read())

if args.reduce:
    # We're running wasm-tools shrink inside the script that collects the output of both runtimes. This is a bit
    # backwards, so to reorder things, we tell wasm-reduce to run this script in --compare mode, while passing
    # along the rest of the arguments
    #
    # wasm-tools API:
    # The predicate script is given a Wasm file as its first and only argument.

    callback_script = (f"#! /usr/bin/env bash\n" +
                       f"\n" +
                       f"# This callback script was AUTOMATICALLY GENERATED by compare.py\n" +
                       f"./compare.py --compare \\\n" +
                       f"  --config {args.config} \\\n" +
                       f"  --wasmlike {args.wasmlike} \\\n" +
                       f"  --reducer '{args.reducer}' \\\n" +
                       f"  --program $1 \\\n" +
                       f"  --timeout {args.timeout} \\\n" +
                       f"  --runtime-a \"{args.runtime_a}\" \\\n" +
                       f"  --runtime-b \"{args.runtime_b}\" \\\n"
                       )
    if args.runtime_c:
        callback_script += f"  --runtime-c \"{args.runtime_c}\" \\\n"

    with open('callback.sh', 'w') as f:
        f.write(callback_script)
    os.chmod('callback.sh', os.stat('callback.sh').st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

    print("Starting reducer.")
    subprocess.run(shlex.split(f"{args.reducer} --output reduced_result.wasm ./callback.sh test_wasm.wasm"))

    ### Debugging aid (trying to debug a subprocess is not a fun can of worms)
    # wasm-tools shrink --output result_wasm.wasm './compare.py --compare --config ~/harness/configs/wasm-all-config.yml -a "NodeJS" -b "Wasmer llvm/universal/optimize" -p test_wasm.wasm' test_wasm.wasm
    #                                f"--output result_wasm.wasm " +
    #                                f"'./compare.py --compare " +
    #                                f"--config {args.config} " +
    #                                f"--runtime-a \"{args.runtime_a}\" " +
    #                                f"--runtime-b \"{args.runtime_b}\" " +
    #                                f"--wasmlike {args.wasmlike} " +
    #                                f"--reducer {args.reducer} " +
    #                                f"--program test_wasm.wasm " +
    #                                f"--timeout {args.timeout}'" +
    #                                f"test_wasm.wasm"))
    # derp = 0 
