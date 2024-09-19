# WebAssembly Sandbox

This repository is the primary host of Wasmlike, a fuzzer that randomly generates semantically valid
WebAssembly programs.

### Testing Methodology

Wasmlike is focused on finding wrong code defects in systems under test using differential testing.
To do so, the programs it generates share some common characteristics: a main function that always
returns an `i32`, one page of memory, and a set of functions to return global variable values to the
host. All of this information is combined into a single CRC value to allow comparing the results of
different systems under test.

### Systems Under Test

To date, Wasmlike supports the following systems under test:

|      System       | Compiler  | Optimization |
| :---------------: | :-------: | :----------: |
|  Node.js 16.16.0  |     -     |      -       |
|   Wasmer 4.0.0    | Cranelift |      No      |
|   Wasmer 4.0.0    | Cranelift |     Yes      |
|   Wasmer 4.0.0    |   LLVM    |      No      |
|   Wasmer 4.0.0    |   LLVM    |     Yes      |
|  Wasmtime 10.0.1  |     -     |      No      |
|  Wasmtime 10.0.1  |     -     |     Yes      |
| Firefox (stable)  |     -     |      -       |
| Chromium (stable) |     -     |      -       |



### Repository Structure

Each group of systems under test has its own directory, containing the test driver and other
necessary files. The browsers are under `playwright`, since that is the automated test software
responsible for running each browser process.  Node.js and the browsers have their test drivers
written in javascript. Wasmer and Wasmtime have their test drivers written in Rust. 

The directory `crc-debug` contains files designed to test CRC parity between different test drivers,
as well as further instructions on what each file is used for and two language projects for
Javascript and Rust APIs.

Notes and interesting annotated Wasm programs are in the `notes` directory

Tools for reduction are in the `reduce` directory. In particular, the python script `compare.py` is
much more advanced than the shell script `compare.sh`, which is kept for historical reasons and if
Binaryen's reducer/optimizer ever supports more modern instructions.

Finally, the `wasmlike` directory is where Wasmlike is located. It contains `wasmlike.rkt` itself,
as well as some utility scripts to quickly test changes to Wasmlike.



## Wasmlike

Wasmlike is a fuzzer built on Xsmith, a racket library for creating fuzz testers. Wasmlike uses
Xsmith's domain specific language, or DSL, to specify the grammar, types, constraints, and features
of the WebAssembly language. The documentation for Xsmith can be found here:
https://docs.racket-lang.org/xsmith/index.html. 

The general design of Wasmlike is to generate a tree that is always balanced in order to satisfy the
WebAssembly requirement of no undeclared values on the stack at the end of a function or block. To
that end, a few AST nodes representing instructions that do not push exactly one value on the stack
are adjusted so that the end result of the node does push exactly one value. For example: a memory
store instruction consumes a value and does not push one. The AST node that generates this
instruction will render as a memory store as well as some subtree that will generate one value on
the stack (it could be a literal, or it could be a whole chain of operations).

In rough order, the sections of Wasmlike are as follows:
  - Choice weights
  - Grammar specification
  - Feature controls
  - Fresh node generation rules for nodes that need it
  - Depth and recursion limiters
  - Type system specification
  - Rendering/printing

Features and code related to various parts of Wasmlike are generally kept close to where they are
used. However, any 'decision' related code should be kept out of the rendering if possible. For
example, local variables and global variables are the same AST node type, with the only difference
being where their parent is located. Global variables have an initialized value that needs to be
filled. Instead of choosing that value at render time, Wasmlike chooses that value every time
a variable is made, regardless of whether it is a local or a global variable. Then, at render time,
it simply omits the initializing syntax if it is a local varable.

## Building and Running

Building and running wasmlike requires a somewhat up to date installation of Racket. Installation
instructions and downloads for Racket can be found here: https://racket-lang.org.

To build, navigate to `webassembly-sandbox/wasmlike`, and run `raco make wasmlike.rkt`
To run: `racket wasmlike.rkt <options>`. To see the available options, run 
`racket wasmlike.rkt --help`. To see wasmlike in action, run `./quick-test` in the `wasmlike`
directory. The script will run wasmlike, clean up the output for display, compile and run the
program, and display the result. Examining this script and the related tools is a wonderful jumping
off point for understanding the different pieces of machinery that make the whole project work.



