# RGFuzz

This repository contains the artifact (source, data, documentation) to reproduce the results of the paper: "RGFuzz: Fuzzing WebAssembly Runtimes Using Compiler Rules", IEEE S&P 2025 (to appear).

## Contents

- Build & Run
- Evaluation
- Repository Structure
- Guide for new wasmtime version
- Data

## Build & Run

### Build

For each target (wasmtime, wasmer, wasmedge, v8, spidermonkey, jsc), there is a docker image that builds the target. The following commands automatically builds all targets including all images needed for evaluation:

```sh
cd docker
./build.sh
```

If docker images do not build properly, you may need to initialize submodules with the following:

```sh
git submodule init
git submodule update --recursive
cd targets/wasmtime/wasmtime
git submodule init
git submodule update --recursive
cd ../../..
```

If you want to manually build docker images for a specific target, you can run the following:

```sh
cd docker
export UID="$(id -u)"
export GID="$(id -g)"
docker-compose build rgfuzz-base rgfuzz-[target_name]
```

For example, you can run `docker-compose build rgfuzz-wasmtime` to build the docker image for the wasmtime target.

There are also docker images purposed for evaluation (RQ1 and RQ2). These images are the essentially the same with `rgfuzz-[target_name]` images, but they are limited in memory and CPU usage. You can build them individually by running the following:

```sh
cd docker
export UID="$(id -u)"
export GID="$(id -g)"
docker-compose build rgfuzz-base rgfuzz-[target_name]-single
```

For the diversity evaluation (RQ3), you can build the following:

```sh
cd docker
export UID="$(id -u)"
export GID="$(id -g)"
docker-compose build rgfuzz-fairness
```

### Run

You can run the following script with a proper target name. The script automatically runs the container. The command is the following:

```sh
cd docker
./run.sh rgfuzz-[target_name]
```

For example, you can run:

```sh
./run.sh rgfuzz-wasmtime
```

After getting into docker, you can run the following:

```sh 
cd /fuzz/fuzz/executor
python3 main.py --save-crashes --engine-name=wasmtime
```

There are multiple options you can tweak. To get the list of options, you can run the following:

```sh
python3 main.py --help
```

By default, the working directory is located at `/fuzz/run`. The crash directory is located at `/fuzz/crashes`. You'll also see stdout logs in the console as the following (it may take initial ~20 minutes to see the stdout logs):

```
[*] 0: seed: 4268196420 / execution speed: 0.0041690257709439535execs/s / compile success: True / compile failing archs: [('arm64', 0), ('arm64', 1), ('arm64', 2)] / compare result: True / instructions: [('i64.const', (-255,)), ('local.get', ('arg0',))]
[*] 100: seed: 937850723 / execution speed: 0.40411746960923367execs/s / compile success: True / compile failing archs: [('arm64', 0), ('arm64', 1), ('arm64', 2)] / compare result: True / instructions: [('local.get', ('arg1',)), ('local.get', ('arg0',)), ('br_if', (0,))]
[*] 200: seed: 3782451467 / execution speed: 0.7842064973088364execs/s / compile success: True / compile failing archs: [('arm64', 0), ('arm64', 1), ('arm64', 2), ('s390x', 0), ('s390x', 2)] / compare result: True / instructions: [('f32.const', (-1.0,)), ('local.get', ('arg1',)), ('local.get', ('arg0',)), ('i64.const', (42,)), ('local.get', ('arg0',)), ('i64.rem_u', ()), ('i64.le_s', ()), ('block', (0,)), ('drop', ()), ('f32.demote_f64', ()), ('f32.floor', ()), ('i32.const', (-2147483647,)), ('f32.convert_i32_u', ()), ('f32.add', ()), ('end', ()), ('f32.floor', ()), ('f32.div', ())]
(...)
```

To manually inspect the crashes, you can check `log` file in each crash directory. You can also diff `.cur.exec.[arch].[opt]` files to see the difference between the execution results of different architectures and optimization levels.

To modify the config outside the run options, you can modify `config.py` in `fuzz/executor`. You can set additional config options for the targets, modify dictionaries for constants, and tweak code generation probabilities. Outside them, you can also modify the installation paths of your targets, locating the target binaries.

## Evaluation

### Scripts

After building all docker images for evaluation (See the build section), you can run the evaluation scripts in the `eval` directory.

#### RQ1/2: Coverage

_**(PLEASE READ ALL THE INSTRUCTIONS BEFORE RUNNING THE SCRIPTS)**_

`eval/cov/eval.sh` script is designed to run all the evaluation for RQ1 and RQ2. You can run the following script to reproduce the coverage evaluation. You need to build the docker image for the coverage evaluation before running this.

```sh
cd eval/cov/
./eval.sh
```

However, the script will require a total of 110 cores to run, which is not feasible for most systems. You can individually run the scripts for each target with `eval/cov/[target_name]-all-singlecore.sh`. For example, you can run the following:

```sh
cd eval/cov/
./wasmtime-all-singlecore.sh
```

To further reduce the number of simultaneous evaluations, you should modify the script by commenting out some lines starting with `docker compose run`. Each line would use up one CPU core.

We recommend you to decrease the number of simultaneous evaluations to `[number of CPU cores] / 2` to prevent the system from freezing. For example, in our machine, we have 96 cores, so we set the number of simultaneous evaluations to 48.

After the script finishes, you can see the results in the `coverage` directory.

#### RQ3: Diversity

You can run the following script to reproduce the diversity evaluation. You need to build the docker image for the diversity evaluation before running this.

```sh
cd eval/diversity/
./eval.sh
```

This script will take a while to finish. After the script finishes, you can see the results in the `coverage` directory.

## Repository Structure

- `data`: Contains the evaluation results of the paper.
- `docker`: Contains the docker scripts to build and run the docker images.
- `eval`: Contains the evaluation scripts for RQ1, RQ2, and RQ3. These scripts will spawn docker containers each running an evaluation session.
- `fuzz`: Contains the source code of RGFuzz.
    - `eval`: Contains the evaluation scripts that will go inside the docker images. These scripts run inside the containers.
    - `extractor`: Contains the source code of the extractor.
    - `executor`: Contains the source code of the RGFuzz fuzzing engine. This also contains reverse stack-based test case generator. RGFuzz is configured using `config.py`.
- `targets`: Contains the source code, build scripts, and harness code of the targets.

## Guide for new wasmtime version

If you want to upgrade the wasmtime version, you will need to write handlers for new compiler directives. (See paper for details) You can refer to `fuzz/extractor/src/isle_inl.rs` for the implementation of the handlers. After you write handlers, you need to register them in `process_internals_one` function with `or_else` function.

To see if new compiler directives are added, you can observe the changes in `wasmtime/cranelift/codegen/src`. Specifically, you need to observe the following 6 files:

- `wasmtime/cranelift/codegen/src/isle_prelude.rs`
- `wasmtime/cranelift/codegen/src/opts.rs`
- `wasmtime/cranelift/codegen/src/isa/x64/lower/isle.rs`
- `wasmtime/cranelift/codegen/src/isa/aarch64/lower/isle.rs`
- `wasmtime/cranelift/codegen/src/isa/s390x/lower/isle.rs`
- `wasmtime/cranelift/codegen/src/isa/riscv64/lower/isle.rs`

The data structures that are used to implement the extractor might be tricky to understand. You can refer to `fuzz/extractor/src/norm.rs` for the data structures used in the extractor. Also, you can read `fuzz/extractor/README.md` for some details about the rule extractor.

There might be changes in the API of wasmtime. You may need to fix `targets/wasmtime/wasmtime-wrapper/src/main.rs` to use the new API.

## Data

Raw data, the result of the evaluation on our machine, is available as a compressed file in ![link](https://drive.google.com/file/d/1Ys3KnLRAgR7NldLYbmneLb9-7QCkr-an/view?usp=sharing) (2.3g, md5 `c763cb4be9c346ba06c06377e0268efd`). You can decompress it and see the results of the evaluation. Before you decompress the file, make sure you have 33g of disk space. We also include the excel files that contain the coverage data.

To reduce the size of the data, we only included the full coverage report only for 24 hour point on 0th try of each evaluation session. To see them, you can look into `data/rqN/*-0/report/24_0_0` of each session. For the rest of the data, we only included the summary of the coverage data, showing coverage for each source files. You can see them in `data/rqN/*/report/*` of each session and time.

