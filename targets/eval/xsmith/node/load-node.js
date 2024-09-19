//#!/usr/bin/env node
// -*- mode: JavaScript -*-
//
// Copyright (c) 2019 The University of Utah
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
//   * Redistributions of source code must retain the above copyright notice,
//     this list of conditions and the following disclaimer.
//
//   * Redistributions in binary form must reproduce the above copyright
//     notice, this list of conditions and the following disclaimer in the
//     documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.

///////////////////////////////////////////////////////////////////////////////

// Based on:
// https://gist.github.com/kripken/59c67556dc03bb6d57052fedef1e61ab
//   and
// http://thecodebarbarian.com/getting-started-with-webassembly-in-node.js.html
//   and
// https://gist.github.com/kanaka/3c9caf38bc4da2ecec38f41ba24b77df

// To debug: node --inspect-brk --experimental-wasm-bigint load-node.js <wasm-file>
//  - Set a breakpoint past the wasm compilation point, but before instantiation.
//  - In the Node sources window, drill down into the wasm files, where you should
//    find your program

const { crc32 } = require('crc');
const fs = require('fs')
const assert = require('assert')

assert('WebAssembly' in global,
        'WebAssembly global object not detected')

function abortStackOverflow(allocSize) {
  console.log('Stack overflow! Attempted to allocate ' + allocSize + ' bytes on the stack, but failed')
  process.exit(1)
}

var crc_start = true;
var crc_value = 0;

function addToCrc(val) {
    var buf
    if (typeof val === 'number') {
        buf = new Buffer.alloc(4)
        buf.writeInt32LE(val)
    } else {
        console.log('Could not crc given value. The value was not an integer')
        return
    }
    if(crc_start) {
      crc_value = crc32(buf);
      crc_start = false;
    } else {
      crc_value = crc32(buf, crc_value);
    }
}

//TODO - check that this is equivalent to the rust side of things
function crcMemory(wasm_memory){
    crc_value = crc32(wasm_memory.buffer, crc_value)
}

// Loads a WebAssembly dynamic library, returns a promise.
// imports is an optional imports object
async function loadWebAssembly(filename) {
  // Fetch the file and compile it
  const wasm_source = fs.readFileSync(filename);
  const importObject = {
    env: {
      addToCrc: addToCrc
    }
  }

  // Not in NodeJS
  //return WebAssembly.instantiateStreaming(wasm_source, importObject);
  wasm_module = WebAssembly.compile(wasm_source)
  return WebAssembly.instantiate(wasm_source, importObject)
}

/* Start script */

assert(process.argv.length === 3,
  "Usage: node load-node.js program.wasm\n"
   + "Found " + process.argv.length + " arguments instead of 3 \n\n")


const wasm = process.argv[2]
const func = '_main'
const crc_globals = '_crc_globals'
const memory = '_memory'
const args = []

loadWebAssembly(wasm)
  .then(({instance, module}) => {
    crc_value = 0 // reset the crc every time this function is run
    var exports = instance.exports
    assert(exports, 'no exports found')
    assert(func in exports, func + ' not found in wasm module exports')
    assert(crc_globals in exports, crc_globals + ' not found in wasm module exports')
    assert(memory in exports, memory + ' not found in wasm module exports')

    // Call wasm main function and add the return value to the crc
    addToCrc(exports[func]())
    // Update crc value with values of all globals. The exported function will call
    // addToCrc on its own
    exports[crc_globals]()
    // Update crc value with contents of memory
    crcMemory(exports[memory]) 
    // Print crc
    console.log(crc_value.toString(16)) 
  })
  .catch(res => {
    console.log(res)
    process.exit(1)
  })

///////////////////////////////////////////////////////////////////////////////

// End of file.

