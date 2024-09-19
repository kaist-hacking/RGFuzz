#!/usr/bin/env python3

import xlwings
import matplotlib.pyplot as plt
import matplotlib
from scipy.stats import mannwhitneyu, rankdata
import numpy as np

NAME = "wasmtime-coverage-data-single-final.xlsx"
postfix='-1core'

cov_book = xlwings.Book(NAME)
opt_line_sheet = cov_book.sheets["Line Coverage - Opt"]
lower_line_sheet = cov_book.sheets["Line Coverage - Lower"]
opt_rule_sheet = cov_book.sheets["Line Coverage - Rule Opt"]
lower_rule_sheet = cov_book.sheets["Line Coverage - Rule Lower"]
total_sheet = cov_book.sheets["Total Coverage"]

x_axis_selector = "C1:CU1"
x_axis = opt_line_sheet.range(x_axis_selector).value

opt_line_fig, opt_line_ax = plt.subplots()
lower_line_fig, lower_line_ax = plt.subplots()
opt_rule_fig, opt_rule_ax = plt.subplots()
lower_rule_fig, lower_rule_ax = plt.subplots()
total_fig, total_ax = plt.subplots()

ax_list = [
    (opt_line_fig, opt_line_ax),
    (lower_line_fig, lower_line_ax),
    (opt_rule_fig, opt_rule_ax),
    (lower_rule_fig, lower_rule_ax),
    (total_fig, total_ax),
]
for fig, ax in ax_list:
    if ax != total_ax:
        ax.set_ylim([0, 100])
    ax.set_xticks(range(len(x_axis))[::8])
    ax.tick_params(axis='x', rotation=45)
    ax.set_xlim([0, len(x_axis)-1])
    ax.margins(x=0)
    ax.legend()
    # fig.set_figwidth(13.33 / 3)
    # fig.set_figheight(7.5 / 3)
    fig.set_figwidth(3.3)
    fig.set_figheight(3.3)
    # fig.subplots_adjust(top=0.8)

# opt_line_ax.set_ylabel("Line Coverage (%)")
# lower_line_ax.set_ylabel("Line Coverage (%)")
# opt_rule_ax.set_ylabel("Rule Coverage (%)")
# lower_rule_ax.set_ylabel("Rule Coverage (%)")
# total_ax.set_ylabel("Line Coverage (%)")
opt_line_ax.set_ylabel("Coverage (%)")

title_pad = 10
matplotlib.rcParams['axes.titlepad'] = title_pad
opt_line_ax.set_title("Optimization Line Coverage")
lower_line_ax.set_title("Lowering Line Coverage")
opt_rule_ax.set_title("Optimization Rule Coverage")
lower_rule_ax.set_title("Lowering Rule Coverage")
total_ax.set_title("Total Line Coverage")

idx = 2
data_last = {'opt_line': [], 'lower_line': [], 'opt_rule': [], 'lower_rule': [], 'total': []} # data at last time points
while True:
    data_selector = f"D{idx}:CU{idx+4}"
    opt_line_data = opt_line_sheet.range(data_selector).value
    lower_line_data = lower_line_sheet.range(data_selector).value
    opt_rule_data = opt_rule_sheet.range(data_selector).value
    lower_rule_data = lower_rule_sheet.range(data_selector).value
    total_data = total_sheet.range(data_selector).value

    if opt_line_data[0][0] == None:
        break
    
    opt_line_min = [min(x) for x in zip(*opt_line_data)]
    opt_line_max = [max(x) for x in zip(*opt_line_data)]
    opt_line_avg = [sum(x) / len(x) for x in zip(*opt_line_data)]
    lower_line_min = [min(x) for x in zip(*lower_line_data)]
    lower_line_max = [max(x) for x in zip(*lower_line_data)]
    lower_line_avg = [sum(x) / len(x) for x in zip(*lower_line_data)]
    opt_rule_min = [min(x) for x in zip(*opt_rule_data)]
    opt_rule_max = [max(x) for x in zip(*opt_rule_data)]
    opt_rule_avg = [sum(x) / len(x) for x in zip(*opt_rule_data)]
    lower_rule_min = [min(x) for x in zip(*lower_rule_data)]
    lower_rule_max = [max(x) for x in zip(*lower_rule_data)]
    lower_rule_avg = [sum(x) / len(x) for x in zip(*lower_rule_data)]
    total_min = [min(x) for x in zip(*total_data)]
    total_max = [max(x) for x in zip(*total_data)]
    total_avg = [sum(x) / len(x) for x in zip(*total_data)]

    name = opt_line_sheet.range(f"A{idx}").value
    opt_line_ax.plot(x_axis, [float('nan')]+opt_line_avg, label=name)
    lower_line_ax.plot(x_axis, [float('nan')]+lower_line_avg, label=name)
    opt_rule_ax.plot(x_axis, [float('nan')]+opt_rule_avg, label=name)
    lower_rule_ax.plot(x_axis, [float('nan')]+lower_rule_avg, label=name)
    total_ax.plot(x_axis, [float('nan')]+total_avg, label=name)

    opt_line_ax.fill_between(x_axis, [float('nan')]+opt_line_min, [float('nan')]+opt_line_max, alpha=0.2)
    lower_line_ax.fill_between(x_axis, [float('nan')]+lower_line_min, [float('nan')]+lower_line_max, alpha=0.2)
    opt_rule_ax.fill_between(x_axis, [float('nan')]+opt_rule_min, [float('nan')]+opt_rule_max, alpha=0.2)
    lower_rule_ax.fill_between(x_axis, [float('nan')]+lower_rule_min, [float('nan')]+lower_rule_max, alpha=0.2)
    total_ax.fill_between(x_axis, [float('nan')]+total_min, [float('nan')]+total_max, alpha=0.2)

    data_last['opt_line'].append((name, [x[-1] for x in opt_line_data]))
    data_last['lower_line'].append((name, [x[-1] for x in lower_line_data]))
    data_last['opt_rule'].append((name, [x[-1] for x in opt_rule_data]))
    data_last['lower_rule'].append((name, [x[-1] for x in lower_rule_data]))
    data_last['total'].append((name, [x[-1] for x in total_data]))

    idx += 8

for fig, ax in ax_list:
    if ax == total_ax:
        ax.set_ylim([0, ax.get_ylim()[1] + 8])
    ax.legend(loc='upper center', ncol = len(ax.lines) / 3, bbox_to_anchor=(0.5, 1), fontsize='small')

print("Mann-Whitney U tests")
for name, data in data_last.items():
    print(f"Data: {name}")
    for i in range(len(data)):
        for j in range(i+1, len(data)):
            print(f"{data[i][0]} vs {data[j][0]}")
            result = mannwhitneyu(data[i][1], data[j][1], alternative='two-sided')
            print(result)
            if result.pvalue < 0.05:
                print("Significant (p < 0.05)")
            else:
                print("Not significant (p >= 0.05)")

# varhga and delaney A tests
# from https://gist.github.com/jacksonpradolima/f9b19d65b7f16603c837024d5f8c8a65
def a12_test(x, y):
    n1 = len(x)
    n2 = len(y)

    ranks = rankdata(x + y)
    r1 = ranks[:n1]
    r2 = ranks[n1:]

    r1_sum = np.sum(r1)
    a12 = (2 * r1_sum - n1 * (n1 + 1)) / (2 * n1 * n2)
    return a12

print("Vargha and Delaney A tests")
for name, data in data_last.items():
    print(f"Data: {name}")
    for i in range(len(data)):
        for j in range(i+1, len(data)):
            print(f"{data[i][0]} vs {data[j][0]}")
            result = a12_test(data[i][1], data[j][1])
            print(f"A12: {result}")

opt_line_fig.show()

opt_line_fig.savefig(f'images/wasmtime-opt-line{postfix}.pdf', bbox_inches='tight')
lower_line_fig.savefig(f'images/wasmtime-lower-line{postfix}.pdf', bbox_inches='tight')
opt_rule_fig.savefig(f'images/wasmtime-opt-rule{postfix}.pdf', bbox_inches='tight')
lower_rule_fig.savefig(f'images/wasmtime-lower-rule{postfix}.pdf', bbox_inches='tight')
total_fig.savefig(f'images/wasmtime-total-line{postfix}.pdf', bbox_inches='tight')

input()