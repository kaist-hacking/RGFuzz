#!/usr/bin/env python3

import os

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(source_dir)

# TODO: write code for executing all cases

with open(os.path.join(root_dir, "result_ext_all.txt"), "rt") as f:
    ext_all = set(f.read().split("\n")[:-1]) # A + B + C

with open(os.path.join(root_dir, "result_ext_typing.txt"), "rt") as f:
    ext_typing = set(f.read().split("\n")[:-1]) # A

with open(os.path.join(root_dir, "result_ext_opt.txt"), "rt") as f:
    ext_opt = set(f.read().split("\n")[:-1]) # B

with open(os.path.join(root_dir, "result_ext_lower.txt"), "rt") as f:
    ext_lower = set(f.read().split("\n")[:-1]) # C

with open(os.path.join(root_dir, "result_ext_opttyping.txt"), "rt") as f:
    ext_opttyping = set(f.read().split("\n")[:-1]) # A + B

with open(os.path.join(root_dir, "result_ext_lowertyping.txt"), "rt") as f:
    ext_lowertyping = set(f.read().split("\n")[:-1]) # A + C

with open(os.path.join(root_dir, "result_ext_optlower.txt"), "rt") as f:
    ext_optlower = set(f.read().split("\n")[:-1]) # B + C

print("A (typing):", len(ext_typing))
print("B (opt):", len(ext_opt))
print("C (lower):", len(ext_lower))
print("A + B:", len(ext_opttyping))
print("A + C:", len(ext_lowertyping))
print("B + C:", len(ext_optlower))
print("A + B + C:", len(ext_all))
print("-"*20)
typing_only_len = len(ext_all) - len(ext_optlower)
opt_only_len = len(ext_all) - len(ext_lowertyping)
lower_only_len = len(ext_all) - len(ext_opttyping)
all_intersect_len = len(ext_typing) + len(ext_opt) + len(ext_lower) + typing_only_len + opt_only_len + lower_only_len - 2*len(ext_all)
typing_opt_only_len = len(ext_typing) + len(ext_opt) - len(ext_opttyping) - all_intersect_len
typing_lower_only_len = len(ext_typing) + len(ext_lower) - len(ext_lowertyping) - all_intersect_len
opt_lower_only_len = len(ext_opt) + len(ext_lower) - len(ext_optlower) - all_intersect_len
print("A only:", typing_only_len)
print("B only:", opt_only_len)
print("C only:", lower_only_len)
print("AB only:", typing_opt_only_len)
print("AC only:", typing_lower_only_len)
print("BC only:", opt_lower_only_len)
print("ABC only:", all_intersect_len)
print("-"*20)
# print("\n".join(ext_all - ext_opttyping))
print(ext_opt.union(ext_lower) - ext_optlower)
print(ext_optlower - ext_opt.union(ext_lower))
