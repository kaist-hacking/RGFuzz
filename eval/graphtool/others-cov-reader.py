#!/usr/bin/env python3

import xlwings
import matplotlib.pyplot as plt
import matplotlib
from scipy.stats import mannwhitneyu, rankdata
import numpy as np

NAME = "coverage-data-single-others-final.xlsx"
postfix='-1core'

cov_book = xlwings.Book(NAME)
wasmer_sheet = cov_book.sheets["wasmer total coverage"]
wasmedge_sheet = cov_book.sheets["wasmedge total coverage"]
v8_sheet = cov_book.sheets["v8 total coverage"]
jsc_sheet = cov_book.sheets["jsc total coverage"]

x_axis_selector = "C1:CU1"
x_axis = wasmer_sheet.range(x_axis_selector).value

wasmer_fig, wasmer_ax = plt.subplots()
wasmedge_fig, wasmedge_ax = plt.subplots()
v8_fig, v8_ax = plt.subplots()
jsc_fig, jsc_ax = plt.subplots()

ax_list = [
    (wasmer_fig, wasmer_ax),
    (wasmedge_fig, wasmedge_ax),
    (v8_fig, v8_ax),
    (jsc_fig, jsc_ax),
]

for fig, ax in ax_list:
    ax.set_xticks(range(len(x_axis))[::8])
    ax.tick_params(axis='x', rotation=45)
    ax.set_xlim([0, len(x_axis)-1])
    ax.margins(x=0)
    ax.legend()
    # fig.set_figwidth(13.33 / 3)
    # fig.set_figheight(7.5 / 3)
    fig.set_figwidth(3.5)
    fig.set_figheight(3.5)
    # fig.subplots_adjust(top=0.8)

wasmer_ax.set_ylabel("Line Coverage (%)")
# wasmedge_ax.set_ylabel("Line Coverage (%)")
# v8_ax.set_ylabel("Line Coverage (%)")
# jsc_ax.set_ylabel("Line Coverage (%)")

title_pad = 10
matplotlib.rcParams['axes.titlepad'] = title_pad
wasmer_ax.set_title("Wasmer")
wasmedge_ax.set_title("Wasmedge")
v8_ax.set_title("V8")
jsc_ax.set_title("JSC")

idx = 2
data_last = {'wasmer': [], 'wasmedge': [], 'v8': [], 'jsc': []} # data at last time points
while True:
    data_selector = f"D{idx}:CU{idx+4}"
    wasmer_data = wasmer_sheet.range(data_selector).value
    wasmedge_data = wasmedge_sheet.range(data_selector).value
    v8_data = v8_sheet.range(data_selector).value
    jsc_data = jsc_sheet.range(data_selector).value

    if wasmer_data[0][0] == None:
        break

    wasmer_min = [min(x) for x in zip(*wasmer_data)]
    wasmer_max = [max(x) for x in zip(*wasmer_data)]
    wasmer_avg = [sum(x) / len(x) for x in zip(*wasmer_data)]
    wasmedge_min = [min(x) for x in zip(*wasmedge_data)]
    wasmedge_max = [max(x) for x in zip(*wasmedge_data)]
    wasmedge_avg = [sum(x) / len(x) for x in zip(*wasmedge_data)]
    v8_min = [min(x) for x in zip(*v8_data)]
    v8_max = [max(x) for x in zip(*v8_data)]
    v8_avg = [sum(x) / len(x) for x in zip(*v8_data)]
    jsc_min = [min(x) for x in zip(*jsc_data)]
    jsc_max = [max(x) for x in zip(*jsc_data)]
    jsc_avg = [sum(x) / len(x) for x in zip(*jsc_data)]

    name = wasmer_sheet.range(f"A{idx}").value
    # if name == "RGFuzz-":
    #     idx += 8
    #     continue

    wasmer_ax.plot(x_axis, [float('nan')]+wasmer_avg, label=name)
    wasmedge_ax.plot(x_axis, [float('nan')]+wasmedge_avg, label=name)
    v8_ax.plot(x_axis, [float('nan')]+v8_avg, label=name)
    jsc_ax.plot(x_axis, [float('nan')]+jsc_avg, label=name)

    wasmer_ax.fill_between(x_axis, [float('nan')]+wasmer_min, [float('nan')]+wasmer_max, alpha=0.2)
    wasmedge_ax.fill_between(x_axis, [float('nan')]+wasmedge_min, [float('nan')]+wasmedge_max, alpha=0.2)
    v8_ax.fill_between(x_axis, [float('nan')]+v8_min, [float('nan')]+v8_max, alpha=0.2)
    jsc_ax.fill_between(x_axis, [float('nan')]+jsc_min, [float('nan')]+jsc_max, alpha=0.2)

    data_last['wasmer'].append((name, [x[-1] for x in wasmer_data]))
    data_last['wasmedge'].append((name, [x[-1] for x in wasmedge_data]))
    data_last['v8'].append((name, [x[-1] for x in v8_data]))
    data_last['jsc'].append((name, [x[-1] for x in jsc_data]))

    idx += 8

for fig, ax in ax_list:
    ylim = ax.get_ylim()
    ax.set_ylim((0, ylim[1] * 1.3))
    # legend location is bugged!!
    ax.legend(loc='upper center', ncol = 2, bbox_to_anchor=(0.5, 1), fontsize='medium')

print("Mann-Whitney U tests")
for name in ['wasmer', 'wasmedge', 'v8', 'jsc']:
    print(name)
    for i in range(len(data_last[name])):
        for j in range(i+1, len(data_last[name])):
            print(f"{data_last[name][i][0]} vs {data_last[name][j][0]}")
            result = mannwhitneyu(data_last[name][i][1], data_last[name][j][1], alternative='two-sided')
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

print("Varhga and Delaney A tests")
for name in ['wasmer', 'wasmedge', 'v8', 'jsc']:
    print(name)
    for i in range(len(data_last[name])):
        for j in range(i+1, len(data_last[name])):
            print(f"{data_last[name][i][0]} vs {data_last[name][j][0]}")
            result = a12_test(data_last[name][i][1], data_last[name][j][1])
            print(f"A12: {result}")

wasmer_fig.show()

wasmer_fig.savefig(f'images/wasmer-total-line{postfix}.pdf', bbox_inches='tight')
wasmedge_fig.savefig(f'images/wasmedge-total-line{postfix}.pdf', bbox_inches='tight')
v8_fig.savefig(f'images/v8-total-line{postfix}.pdf', bbox_inches='tight')
jsc_fig.savefig(f'images/jsc-total-line{postfix}.pdf', bbox_inches='tight')

input()