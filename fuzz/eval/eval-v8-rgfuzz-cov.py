#!/usr/bin/env python3

import os
import sys
import shutil
import threading
import time

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.join(os.path.dirname(source_dir), "executor")
sys.path[0] = root_dir

import config

if __name__ == "__main__":

    root = os.path.dirname(os.path.dirname(root_dir))
    profraw_dir = os.path.join(config.coverage_dir, config.engine_name.replace('-xsmith', ''))
    report_dir = os.path.join(config.coverage_dir, "report")
    backup_dir = os.path.join(config.coverage_dir, "backup")
    
    for path in (profraw_dir, report_dir, backup_dir):
        if not os.path.exists(path):
            os.makedirs(path)
        else:
            if path == backup_dir and input("Remove backup? (Y/N):") != "Y":
                sys.exit(1)
            shutil.rmtree(path)
            os.makedirs(path)
    
    import main

    TIME = 24*60*60 # 24 hours
    INTERVAL = 15*60 # 15 minutes
    # TIME = 20*60 # 24 hours
    # INTERVAL = 10*60 # 15 minutes

    time_hours = TIME // 60 // 60
    time_minutes = TIME // 60 % 60
    time_seconds = TIME % 60
    time_timeset = f"{time_hours}_{time_minutes}_{time_seconds}"

    anchor_time = 0
    fuzz_thread = threading.Thread(target=main.main)
    for cur_time in range(TIME//INTERVAL+1):
        if cur_time == 0:
            fuzz_thread.start()
            anchor_time = time.time()
        
        # copy out profraw files
        if cur_time > 0:
            realtime = cur_time*INTERVAL
            hours = realtime // 60 // 60
            minutes = realtime // 60 % 60
            seconds = realtime % 60
            dirname = os.path.join(backup_dir, f"{hours}_{minutes}_{seconds}")
            shutil.copytree(profraw_dir, dirname, False, None)

        if cur_time == TIME//INTERVAL:
            break
        
        time_delta = time.time() - anchor_time
        time_to_sleep = (cur_time + 1)*INTERVAL - time_delta
        print(time_to_sleep)
        time.sleep(time_to_sleep)
        print(cur_time, time.time())
        print("-"*30)

    main.main_kill_switch = True
    fuzz_thread.join()

    # profraw merge
    for timeset in os.listdir(backup_dir):
        process_dir = os.path.join(backup_dir, timeset)
        arch_dirs = map(lambda x: (x, os.path.join(process_dir, x)), config.wasmtime_arch_list)
        
        for arch, arch_dir in arch_dirs:
            print(timeset, arch)

            profdata_name = os.path.join(report_dir, f"{timeset}_{arch}.profdata")
            args = ["llvm-profdata-19", "merge", "-sparse", "-o", profdata_name, arch_dir]
            os.system(' '.join(args))

            report_dir_name = os.path.join(report_dir, timeset, arch)
            
            compilation_dir = os.path.join(
                root, "targets", "v8", "v8", "out", f"{arch}.release"
            )
            binary_name = os.path.join(compilation_dir, "d8")
            
            args = [
                "llvm-cov-19", 
                "show", 
                f"-output-dir={report_dir_name}", 
                "-format=html", 
                f"-instr-profile={profdata_name}",
                f"-compilation-dir={compilation_dir}",
                binary_name
            ]
            os.system(' '.join(args))
            if timeset != time_timeset:
                shutil.rmtree(os.path.join(report_dir_name, "coverage"))
                os.remove(os.path.join(report_dir_name, "style.css"))
    
    if os.path.exists(config.crash_save_rootdir):
        shutil.copytree(config.crash_save_rootdir, os.path.join(config.coverage_dir, "crashes"))
