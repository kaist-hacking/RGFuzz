### CRC testing instructions:

The main goal of this set of files is to be able to test that the crc results ALWAYS match if no
wrong code bug is present. To this end, there is a set of three wasm programs along with two
language projects.

The three wasm programs each test a different area of generated wasm programs that affect the crc: 
  - The return value from the main function
  - The global variables present in the program
  - The contents of memory

The two language projects are to test that a crc computation in the node-based test driver is the
same as a crc computation in the rust-based test driver. In particular, that the crc polynomial and
other algorithmic factors are the exact same. Currently, all systems under test with Wasmlike either
use a Javascript or a Rust API. If more systems under test are added with a different API, consider
adding a test case to this directory.

