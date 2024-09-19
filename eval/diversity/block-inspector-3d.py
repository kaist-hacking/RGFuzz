#!/usr/bin/env python3

import os
import sys
import argparse
import matplotlib.pyplot as plt
import matplotlib
import numpy as np
from collections import defaultdict 
import math
from mpl_toolkits.mplot3d import axes3d, Axes3D

root_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
coverage_root = os.path.join(root_dir, "coverage")

NAMES = {
    'rgfuzz-fairness-all': 'RGFuzz',
    'wasm-smith-fairness': 'Wasm-smith',
    'xsmith-fairness': 'Xsmith',
}

parser = argparse.ArgumentParser(description='Please put args')
parser.add_argument(
  '--report-paths', 
  dest='report_paths', 
  type=str, 
  default='', 
  required=True,
  help='report path: e.g., rgfuzz-fairness-all / e.g.2., rgfuzz-fairness-all,rgfuzz-fairness-typing'
)
parser.add_argument(
    '--coverage-root',
    dest='coverage_root',
    type=str,
    default=coverage_root,
    help='coverage root path'
)

args = parser.parse_args()
coverage_root = args.coverage_root

XY_SIZE = 20

def add_to_plot(path, ax):
    result_dir = os.path.join(coverage_root, path)
    with open(os.path.join(result_dir, "counts.txt"), "rt") as f:
        counts_raw = f.read().split('\n')

    instr_counts = eval(counts_raw[0])
    btype_counts = eval(counts_raw[2])
    module_valid_cnt = int(counts_raw[5].split(": ")[1])

    total_btype_count = sum(btype_counts.values())
    btype_freqs = defaultdict(float)
    max_x = 0 # params
    max_y = 0 # returns
    for btype, cnt in btype_counts.items():
        btype_freqs[(len(btype[1]), len(btype[2]))] += cnt / total_btype_count
        max_x = max(max_x, len(btype[1]))
        max_y = max(max_y, len(btype[2]))
    
    freq_data = np.zeros((XY_SIZE, XY_SIZE))
    for coords, freq in btype_freqs.items():
        if coords[0] >= XY_SIZE or coords[1] >= XY_SIZE:
            continue
        x = coords[0]
        y = coords[1]
        freq_data[x][y] = freq

    x = np.arange(XY_SIZE)
    y = np.arange(XY_SIZE)
    x, y = np.meshgrid(x, y)
    dz = freq_data.ravel()
    mask_dz = dz == 0
    ax.bar3d(x.ravel()[~mask_dz], y.ravel()[~mask_dz], np.zeros_like(x.ravel()[~mask_dz]), 1.0, 1.0, dz[~mask_dz], shade=True)
    ax.set_xlim((-1,XY_SIZE))
    ax.set_ylim((-1,XY_SIZE))
    ax.invert_yaxis()

for path in args.report_paths.split(','):
    fig = plt.figure()
    fig.set_figwidth(3.5)
    fig.set_figheight(3)
    ax = fig.add_subplot(projection='3d')
    add_to_plot(path, ax)
    ax.set_title(f"{NAMES[path]}")
    ax.set_xlabel('Returns')
    ax.set_ylabel('Params')
    ax.set_zlabel('Frequency')
    fig.subplots_adjust(left=0, right=0.9, top=0.9, bottom=0.1)
    fig.savefig(os.path.join(coverage_root, f"block-{path}.png"))
    fig.savefig(os.path.join(coverage_root, f"block-{path}.pdf"))
