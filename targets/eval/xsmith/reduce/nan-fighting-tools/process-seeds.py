#!/usr/bin/env python3

import subprocess
import sys

# Reads a list of seeds (seeds.txt) and runs nan_test.sh on each one.
# This is a time-saving tool to filter out wrong-code errors that compute NaNs.


with open("seeds.txt") as s:
	lines = s.readlines()
	i = 1
	total = str(len(lines))
	for line in lines:
		if subprocess.run("./nan-test.sh " + line.strip(), shell=True).returncode == 0:
			print(">>> " + line.strip() + " is NaN-free")
		else:
			print("[" + str(i) + "/" + total + "] " + line.strip() + " has NaNs")
		i += 1
