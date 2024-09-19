#!/usr/bin/env python3

# This code evaluates wasmtime differential
# periodically copies and saves corpus

import os
import signal
import sys
import shutil
import threading
import time
import subprocess

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.join(os.path.dirname(source_dir), "executor")
sys.path[0] = root_dir

import config

# ALLOWED_ENGINES = "wasmtime"
ALLOWED_ENGINES = "wasmtime,spec"
killswitch = False
cargo_path = os.path.expanduser(os.path.join("~", ".cargo", "bin", "cargo"))

def run_cargo_fuzz():
    global killswitch

    env = os.environ
    env["ALLOWED_ENGINES"] = ALLOWED_ENGINES
    env["ASAN_OPTIONS"] = "detect_leaks=0" # leak sanitizer is buggy
    
    args = [cargo_path, "fuzz", "run", "--release", "differential"]
    if config.num_processes > 1:
        args.append(f"--jobs={config.num_processes}")
    args += ["--", "-rss_limit_mb=0", "-ignore_crashes=1"]
    cwd = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmtime", "wasmtime", "fuzz")

    start_time = time.time()
    subp = subprocess.Popen(
        args + [f"-max_total_time={int(TIME - (time.time() - start_time))}"], env=env, shell=False, cwd=cwd,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
    )
    while not killswitch:
        poll_result = subp.poll()
        if poll_result != None and poll_result != 0 and TIME - (time.time() - start_time) > 0:
            subp = subprocess.Popen(
                args + [f"-max_total_time={int(TIME - (time.time() - start_time))}"], env=env, shell=False, cwd=cwd,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
            )
        elif poll_result != None and poll_result == 0:
            break

        time.sleep(1)
    subp.wait()

def cov_cargo_fuzz():
    global killswitch

    env = os.environ
    env["ALLOWED_ENGINES"] = ALLOWED_ENGINES
    env["ASAN_OPTIONS"] = "detect_leaks=0" # leak sanitizer is buggy
    
    args = [cargo_path, "fuzz", "coverage", "--release", "differential", "--", "-rss_limit_mb=0"]
    cwd = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmtime", "wasmtime", "fuzz")
    subp = subprocess.Popen(
        args, env=env, shell=False, cwd=cwd,
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE
    )
    while subp.poll() == None:
        print(subp.stderr.readline().decode(), end="", flush=True)
    
    subp.wait()

def build_cargo_fuzz():
    os.system("eval $(opam env --switch=default)")

    env = os.environ
    env["ALLOWED_ENGINES"] = ALLOWED_ENGINES
    env["ASAN_OPTIONS"] = "detect_leaks=0" # leak sanitizer is buggy
    
    args = [cargo_path, "fuzz", "build", "--release", "differential"]
    cwd = os.path.join(os.path.dirname(os.path.dirname(root_dir)), "targets", "wasmtime", "wasmtime", "fuzz")
    subp = subprocess.Popen(
        args, env=env, shell=False, cwd=cwd,
        stdout=subprocess.DEVNULL, stderr=subprocess.PIPE
    )
    while subp.poll() == None and not killswitch:
        print(subp.stderr.readline().decode(), end="", flush=True)
    subp.kill()

if __name__ == "__main__":
    root = os.path.dirname(os.path.dirname(root_dir))
    
    corpus_dir = os.path.join(root, "targets", "wasmtime", "wasmtime", "fuzz", "corpus", "differential")
    coverage_dir = os.path.join(root, "targets", "wasmtime", "wasmtime", "fuzz", "coverage", "differential")
    report_dir = os.path.join(config.coverage_dir, "report")
    backup_dir = os.path.join(config.coverage_dir, "backup")
    
    for path in (corpus_dir, coverage_dir, report_dir, backup_dir):
        if not os.path.exists(path):
            os.makedirs(path)
        else:
            if path == backup_dir and input("Remove backup? (Y/N):") != "Y":
                sys.exit(1)
            shutil.rmtree(path)
            os.makedirs(path)
    
    TIME = 24*60*60 # 24 hours
    INTERVAL = 15*60 # 15 minutes
    # TIME = 3*10 # 24 hours
    # INTERVAL = 1*10 # 15 minutes

    time_hours = TIME // 60 // 60
    time_minutes = TIME // 60 % 60
    time_seconds = TIME % 60
    time_timeset = f"{time_hours}_{time_minutes}_{time_seconds}"

    corpus_listed_files = []
    anchor_time = 0
    fuzz_thread = threading.Thread(target=run_cargo_fuzz)

    build_cargo_fuzz()
    corpus_files_dir = os.path.join(backup_dir, "files")
    if os.path.exists(corpus_files_dir):
        shutil.rmtree(corpus_files_dir)
    os.makedirs(corpus_files_dir)
    
    for cur_time in range(TIME//INTERVAL+1):
        if cur_time == 0:
            fuzz_thread.start()
            anchor_time = time.time()
        
        # copy out corpus files
        if cur_time > 0:
            realtime = cur_time*INTERVAL
            hours = realtime // 60 // 60
            minutes = realtime // 60 % 60
            seconds = realtime % 60
            fname = os.path.join(backup_dir, f"{hours}_{minutes}_{seconds}")
            file_names = []
            for file_name in os.listdir(corpus_dir):
                try:
                    shutil.copyfile(os.path.join(corpus_dir, file_name), os.path.join(corpus_files_dir, file_name))
                    file_names.append(file_name)
                except:
                    continue
            with open(fname, "wt") as f:
                f.write(str(file_names))
        
        if cur_time == TIME//INTERVAL:
            break

        time_delta = time.time() - anchor_time
        time_to_sleep = (cur_time + 1)*INTERVAL - time_delta
        print(time_to_sleep)
        time.sleep(time_to_sleep)
        print(cur_time, time.time())
        print("-"*30)
    
    killswitch = True
    fuzz_thread.join()

    # profraw extraction
    cur_cwd = os.getcwd()
    os.chdir(os.path.join(root, "targets", "wasmtime", "wasmtime", "fuzz"))
    for cur_time in list(range(INTERVAL, TIME+INTERVAL, INTERVAL))[::-1]:
        hours = cur_time // 60 // 60
        minutes = cur_time // 60 % 60
        seconds = cur_time % 60
        timeset = f"{hours}_{minutes}_{seconds}"

        # symlink saved corpus
        if os.path.exists(corpus_dir):
            shutil.rmtree(corpus_dir)
        os.makedirs(corpus_dir)
        with open(os.path.join(backup_dir, f"{hours}_{minutes}_{seconds}"), "rt") as f:
            fname_list = eval(f.read())
            for fname in fname_list:
                shutil.copyfile(os.path.join(corpus_files_dir, fname), os.path.join(corpus_dir, fname))
        
        # run coverage
        coverage_file = os.path.join(coverage_dir, "coverage.profdata")
        if os.path.exists(coverage_file):
            os.remove(coverage_file)
        
        cov_cargo_fuzz()

        assert os.path.exists(coverage_file)
        if not os.path.exists(os.path.join(backup_dir, "coverages")):
            os.makedirs(os.path.join(backup_dir, "coverages"))
        profdata_name = os.path.join(backup_dir, "coverages", f"{timeset}.profdata")
        shutil.copyfile(coverage_file, profdata_name)

        report_dir_name = os.path.join(report_dir, timeset)
        compilation_dir = os.path.join(
            root, "targets", "wasmtime", "wasmtime", 
            "fuzz", "target", "x86_64-unknown-linux-gnu", 
            "coverage", "release"
        )
        binary_name = os.path.join(
            root, "targets", "wasmtime", "wasmtime", 
            "fuzz", "target", "x86_64-unknown-linux-gnu", 
            "coverage", "x86_64-unknown-linux-gnu", "release",
            "differential"
        )
        args = [
            "cargo-cov", 
            "--",
            "show", 
            f"-output-dir={report_dir_name}", 
            "-format=html", 
            f"-instr-profile={profdata_name}",
            f"-compilation-dir={compilation_dir}",
            binary_name
        ]
        os.system(' '.join(args))
        # if timeset != time_timeset:
        #     shutil.rmtree(os.path.join(report_dir_name, "coverage"))
        #     os.remove(os.path.join(report_dir_name, "style.css"))
    
    os.chdir(cur_cwd)
    shutil.copytree(
        os.path.join(root, "targets", "wasmtime", "wasmtime", "fuzz", "artifacts"),
        os.path.join(config.coverage_dir, "artifacts")
    )