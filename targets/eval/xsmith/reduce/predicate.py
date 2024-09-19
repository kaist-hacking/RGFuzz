#!/usr/bin/env python3

import subprocess
import re
import argparse
import sys

parser = argparse.ArgumentParser(
                    prog="predicate.py",
                    description="Determines if a program generated by the reducer or wasmlike ever computes a NaN float")


parser.add_argument("filepath")
parser.add_argument("-d", "--debug", action="store_true")

args=parser.parse_args()

# name of temporary file to be fed into wizard-engine
copy_name = "predicate_friendly"

# convert to wat if necessary
correct_path = args.filepath
if args.filepath[-5:] == ".wasm":
    correct_path = args.filepath[:-5] + ".wat"
    subprocess.run("wasm2wat " + args.filepath + " -o " + correct_path, shell=True)

contents = ""
with open(correct_path, 'r') as f:
    contents = f.read()

if args.debug:
    print("\nInputted file:\n" + contents)

# make the program suitable for wizard-engine
contents = re.sub("\(import \"env\" \"addToCrc\" \(func.*\)\)", "(func (result i32) i32.const 0)", contents, count=1)
contents = re.sub("\(export \"_main\"", "(export \"main\"", contents, count=1)
contents = re.sub("(call\s+\$addToCrc)|(call\s+0)", "drop", contents)

if args.debug:
    print("\nModified input to wizard engine:\n" + contents)

with open(copy_name + ".wat", 'w') as f:
    f.write(contents)

# compile the copy
subprocess.run("wat2wasm " + copy_name + ".wat", shell=True)

# run wizard-engine
wizeng_cmd = "wizeng.x86-linux -tio " + copy_name + ".wasm"
wizeng_out = subprocess.run(wizeng_cmd, shell=True, text=True, capture_output=True).stdout

if args.debug:
    print("\nwizard-engine output:\n" + wizeng_out)

# look for NaNs
nan_regex = re.compile(r"(f64:(?![F7]FF0000000000000)[F7]FF[0-9A-F]{13})|(f32:(?![7F]F800000)[7F]F[8-9A-F][0-9A-F]{5})")
match = nan_regex.search(wizeng_out)

subprocess.run("rm predicate_friendly.wasm predicate_friendly.wat", shell=True)

if args.debug:
    if match:
        print("NaNs found:")
        for m in nan_regex.finditer(wizeng_out):
            print(m[0] + " at index " + str(m.start()))
    else:
        print("No NaNs found on stack or as instruction arguments")

if match:
    sys.exit(1)
else: 
    sys.exit(0)