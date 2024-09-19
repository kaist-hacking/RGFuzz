#!/usr/bin/env python3

import os
import sys
from bs4 import BeautifulSoup
import argparse
import itertools
import multiprocessing

root_dir = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
coverage_root = os.path.join(root_dir, "coverage")

parser = argparse.ArgumentParser(description='Please put args')
parser.add_argument(
  '--report-path-pat', 
  dest='report_path_pat', 
  type=str, 
  default='', 
  help='report path: e.g., wasmtime-differential-[idx]/report/[timeset]'
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

def timeset_sort(timeset):
    hours, minutes, seconds = timeset.split('_')
    return int(hours) * 3600 + int(minutes) * 60 + int(seconds)

def get_coverages(cur_tuple):
    idx, timeset = cur_tuple
    report_path = args.report_path_pat.replace('[idx]', str(idx)).replace('[timeset]', timeset)
    report_path = os.path.join(coverage_root, report_path)
    report_html = os.path.join(report_path, 'index.html')
    with open(report_html, 'rt') as f:
        html_content = f.read()
    
    BS = BeautifulSoup(html_content, 'html.parser')

    def filter_opt(href):
        return href and "isle_opt.rs" in href
    entries = BS.find_all(href=filter_opt)
    assert len(entries) == 1
    entry = entries[0].parent.parent.parent
    coverage_entries = list(entry.children)[1:4]
    coverages = list(map(lambda x: x.pre.contents[0].strip().split(' ')[0][:-1], coverage_entries))
    opt_line_coverage = coverages[1]

    opt_html_path = os.path.join(report_path, "coverage", f"{entries[0].contents[0]}.html")
    with open(opt_html_path, "rt") as f:
        opt_html_content = f.read()

    def filter_lower(href):
        return href and "isle_x64.rs" in href
    entries = BS.find_all(href=filter_lower)
    assert len(entries) == 1
    entry = entries[0].parent.parent.parent
    coverage_entries = list(entry.children)[1:4]
    coverages = list(map(lambda x: x.pre.contents[0].strip().split(' ')[0][:-1], coverage_entries))
    lower_line_coverage = coverages[1]

    lower_html_path = os.path.join(report_path, "coverage", f"{entries[0].contents[0]}.html")
    with open(lower_html_path, "rt") as f:
        lower_html_content = f.read()

    def filter_line(content):
        return content and ".isle line " in content
    
    BS_OPT = BeautifulSoup(opt_html_content, 'html.parser')
    BS_LOWER = BeautifulSoup(lower_html_content, 'html.parser')
    opt_entries = BS_OPT.find_all(string=filter_line)
    lower_entries = BS_LOWER.find_all(string=filter_line)

    opt_rules_coverage = {}
    lower_rules_coverage = {}
    for entry in opt_entries:
        is_uncovered = entry.parent.has_attr('class') and entry.parent["class"][0] == "red"
        fname = str(entry).split(' line ')[0].split('/')[-1]
        line_num = int(str(entry).split(' ')[-1][:-1])

        if not fname in opt_rules_coverage:
            opt_rules_coverage[fname] = [[], []]

        if is_uncovered:
            opt_rules_coverage[fname][1].append(line_num)
        else:
            opt_rules_coverage[fname][0].append(line_num)

    for entry in lower_entries:
        is_uncovered = entry.parent.has_attr('class') and entry.parent["class"][0] == "red"
        fname = str(entry).split(' line ')[0].split('/')[-1]
        line_num = int(str(entry).split(' ')[-1][:-1])

        if not fname in lower_rules_coverage:
            lower_rules_coverage[fname] = [[], []]

        if is_uncovered:
            lower_rules_coverage[fname][1].append(line_num)
        else:
            lower_rules_coverage[fname][0].append(line_num)
    
    total_entry = list(BS.find_all(string="Totals")[0].parent.parent.parent.children)[2].contents[0]
    total_coverage = total_entry.contents[0].strip().split(' ')[0][:-1]
    
    return opt_line_coverage, lower_line_coverage, opt_rules_coverage, lower_rules_coverage, total_coverage

for idx in itertools.count(0):
    report_folder = os.path.join(coverage_root, args.report_path_pat.split('/')[0].replace('[idx]', str(idx)))
    if not os.path.exists(report_folder):
        print(f"[*] Folder {report_folder} doesn't exist - exiting...")
        break

    report_timeset_path = args.report_path_pat.replace('[idx]', str(idx))
    report_timeset_path = '/'.join(report_timeset_path.split('/')[:report_timeset_path.split('/').index('[timeset]')])
    report_timeset_path = os.path.join(coverage_root, report_timeset_path)

    line_opt_coverages = []
    rule_opt_coverages = []
    line_lower_coverages = []
    rule_lower_coverages = []
    total_coverages = []

    timesets = filter(lambda x: os.path.isdir(os.path.join(report_timeset_path, x)), os.listdir(report_timeset_path))
    timesets = sorted(timesets, key=timeset_sort)
    
    print("Parallelizing:", multiprocessing.cpu_count())
    with multiprocessing.Pool(multiprocessing.cpu_count()) as pool:
        pool_iter = pool.imap(get_coverages, [(idx, timeset) for timeset in timesets])

        for results in pool_iter:
            line_opt_coverage, line_lower_coverage, opt_rules_coverage, lower_rules_coverage, total_coverage = results
            line_opt_coverages.append(line_opt_coverage)
            line_lower_coverages.append(line_lower_coverage)

            total_covered = 0
            total_uncovered = 0
            for fname in opt_rules_coverage:
                total_cnt = len(opt_rules_coverage[fname][0]) + len(opt_rules_coverage[fname][1])
                covered_cnt = len(opt_rules_coverage[fname][0])
                total_covered += covered_cnt
                total_uncovered += total_cnt - covered_cnt
            total_opt_coverage = "%.2f" % (total_covered / (total_covered + total_uncovered) * 100)
            rule_opt_coverages.append(total_opt_coverage)

            total_covered = 0
            total_uncovered = 0
            for fname in lower_rules_coverage:
                total_cnt = len(lower_rules_coverage[fname][0]) + len(lower_rules_coverage[fname][1])
                covered_cnt = len(lower_rules_coverage[fname][0])
                total_covered += covered_cnt
                total_uncovered += total_cnt - covered_cnt
            total_lower_coverage = "%.2f" % (total_covered / (total_covered + total_uncovered) * 100)
            rule_lower_coverages.append(total_lower_coverage)

            total_coverages.append(total_coverage)

    if len(timesets) == 0:
        continue
    last_timeset = timesets[-1]
    report_path = args.report_path_pat.replace('[idx]', str(idx)).replace('[timeset]', last_timeset)
    report_path = os.path.join(coverage_root, report_path)
    
    print("="*30)
    print("SUMMARY OF IDX", idx)
    print("OPT LINE COVERAGE  : ", ' '.join(line_opt_coverages))
    print("OPT RULE COVERAGE  : ", ' '.join(rule_opt_coverages))
    print("LOWER LINE COVERAGE: ", ' '.join(line_lower_coverages))
    print("LOWER RULE COVERAGE: ", ' '.join(rule_lower_coverages))
    print("TOTAL LINE COVERAGE: ", ' '.join(total_coverages))
    print("-"*20)
    print("OPT RULES COVERAGE")
    total_covered = 0
    total_uncovered = 0
    for fname in opt_rules_coverage:
        total_cnt = len(opt_rules_coverage[fname][0]) + len(opt_rules_coverage[fname][1])
        covered_cnt = len(opt_rules_coverage[fname][0])
        total_covered += covered_cnt
        total_uncovered += total_cnt - covered_cnt
        coverage = "%.2f" % (covered_cnt / total_cnt * 100)
        print(f"{fname}: {coverage}% ({covered_cnt}/{total_cnt})")
        covered_str = ' '.join([str(x) for x in sorted(opt_rules_coverage[fname][0])])
        uncovered_str = ' '.join([str(x) for x in sorted(opt_rules_coverage[fname][1])])
        print(f"{fname}: {covered_str} / {uncovered_str}")
    total_coverage = "%.2f" % (total_covered / (total_covered + total_uncovered) * 100)
    print(f"TOTAL: {total_coverage}% ({total_covered}/{total_covered+total_uncovered})")
    print("-"*20)
    print("LOWER RULES COVERAGE")
    total_covered = 0
    total_uncovered = 0
    for fname in lower_rules_coverage:
        total_cnt = len(lower_rules_coverage[fname][0]) + len(lower_rules_coverage[fname][1])
        covered_cnt = len(lower_rules_coverage[fname][0])
        total_covered += covered_cnt
        total_uncovered += total_cnt - covered_cnt
        coverage = "%.2f" % (covered_cnt / total_cnt * 100)
        print(f"{fname}: {coverage}% ({covered_cnt}/{total_cnt})")
        covered_str = ' '.join([str(x) for x in sorted(lower_rules_coverage[fname][0])])
        uncovered_str = ' '.join([str(x) for x in sorted(lower_rules_coverage[fname][1])])
        print(f"{fname}: {covered_str} / {uncovered_str}")
    total_coverage = "%.2f" % (total_covered / (total_covered + total_uncovered) * 100)
    print(f"TOTAL: {total_coverage}% ({total_covered}/{total_covered+total_uncovered})")
    print("="*30)