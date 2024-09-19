#!/usr/bin/env python3

import os
import sys
import argparse
import matplotlib.pyplot as plt
import matplotlib

root_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
coverage_root = os.path.join(root_dir, "coverage")

NAMES = {
    'rgfuzz-ir-fairness-all': 'RGFuzz',
    'wasm-smith-ir-fairness': 'Wasm-smith',
    'xsmith-ir-fairness': 'Xsmith',
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
parser.add_argument(
    '--cumulative',
    dest='cumulative',
    action=argparse.BooleanOptionalAction,
    type=bool,
    default=True,
    help='is cumulative'
)

args = parser.parse_args()
coverage_root = args.coverage_root

def get_total_keys(paths):
    keys = set()
    for path in paths:
        result_dir = os.path.join(coverage_root, path)
        with open(os.path.join(result_dir, "counts.txt"), "rt") as f:
            counts_raw = f.read()
        ir_counts = eval(counts_raw)
        keys.update(ir_counts.keys())
    return keys

def add_to_plot(path, ax, total_keys):
    result_dir = os.path.join(coverage_root, path)
    with open(os.path.join(result_dir, "counts.txt"), "rt") as f:
        counts_raw = f.read()

    ir_counts = eval(counts_raw)
    # ir_counts.update({x: 0 for x in total_keys - set(ir_counts.keys())})
    ir_counts = sorted(ir_counts.items(), key=lambda x: x[1])
    # iqr
    q1 = ir_counts[int(len(ir_counts) * 0.25)][1]
    q3 = ir_counts[int(len(ir_counts) * 0.75)][1]
    iqr = q3 - q1
    ir_counts = [(x, y) for x, y in ir_counts if q1 - 1.5 * iqr <= y <= q3 + 1.5 * iqr]
    print(ir_counts)

    total_ir_count = sum([x for _, x in ir_counts])
    ir_freqs = [(x, y / total_ir_count) for x, y in ir_counts]
    x_axis = [0.0]
    y_axis = [0.0]
    y_cumulative = 0.0
    for x in range(len(ir_freqs)):
        x_axis.append((x + 1) / len(ir_freqs))
        y_cumulative += ir_freqs[x][1]
        if args.cumulative:
            y_axis.append(y_cumulative)
        else:
            y_axis.append(ir_freqs[x][1])
    
    ax.plot(x_axis, y_axis, label=NAMES[path])

fig, ax = plt.subplots()
fig.set_figwidth(3.5)
fig.set_figheight(3.5)
paths = args.report_paths.split(',')
total_keys = get_total_keys(paths)
for path in paths:
    add_to_plot(path, ax, total_keys)
ax.legend()
ax.set_xlabel("IR Expressions (1/|IR| each)")
if args.cumulative:
    ax.set_title("IR Expressions - IQR")
    # ax.set_ylabel("Cumulative Frequency")
    fig.savefig(os.path.join(coverage_root, "cumul-ir-iqr.png"), bbox_inches = 'tight')
    fig.savefig(os.path.join(coverage_root, "cumul-ir-iqr.pdf"), bbox_inches = 'tight')
else:
    ax.set_title("IR Expressions - IQR")
    # ax.set_ylabel("Frequency")
    fig.savefig(os.path.join(coverage_root, "ir-iqr.png"), bbox_inches = 'tight')
    fig.savefig(os.path.join(coverage_root, "ir-iqr.pdf"), bbox_inches = 'tight')