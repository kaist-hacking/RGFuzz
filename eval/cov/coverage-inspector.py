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

    line_entry = list(BS.find_all(string="Totals")[0].parent.parent.parent.children)[2].contents[0]
    line_coverage = line_entry.contents[0].strip().split(' ')[0][:-1]
    
    return line_coverage

for idx in itertools.count(0):
    report_folder = os.path.join(coverage_root, args.report_path_pat.split('/')[0].replace('[idx]', str(idx)))
    if not os.path.exists(report_folder):
        print(f"[*] Folder {report_folder} doesn't exist - exiting...")
        break

    report_timeset_path = args.report_path_pat.replace('[idx]', str(idx))
    report_timeset_path = '/'.join(report_timeset_path.split('/')[:report_timeset_path.split('/').index('[timeset]')])
    report_timeset_path = os.path.join(coverage_root, report_timeset_path)

    total_line_coverages = []

    timesets = filter(lambda x: os.path.isdir(os.path.join(report_timeset_path, x)), os.listdir(report_timeset_path))
    timesets = sorted(timesets, key=timeset_sort)
    
    print("Parallelizing:", multiprocessing.cpu_count())
    with multiprocessing.Pool(multiprocessing.cpu_count()) as pool:
        pool_iter = pool.imap(get_coverages, [(idx, timeset) for timeset in timesets])

        for results in pool_iter:
            total_line_coverage = results
            total_line_coverages.append(total_line_coverage)

    if len(timesets) == 0:
        continue
    last_timeset = timesets[-1]
    report_path = args.report_path_pat.replace('[idx]', str(idx)).replace('[timeset]', last_timeset)
    report_path = os.path.join(coverage_root, report_path)
    
    print("="*30)
    print("SUMMARY OF IDX", idx)
    print("TOTAL LINE COVERAGE  : ", ' '.join(total_line_coverages))
    print("="*30)