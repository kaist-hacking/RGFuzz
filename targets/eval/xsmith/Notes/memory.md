# Memory -- Jan 9, 2020

Memory operations have 4 parts:

1. The instruction
2. The base address
3. The offset
4. The alignment



The parts go together in a wasm program like this:

	baseaddr
	i32.load/store offset align




### The instruction

> Pretty self explanatory. There are variants for 32 and 64 bit integers and floating 
  point numbers, as well as loading or storing subsets of the bits for particular data-types


### The base address

> This is one half of the address to access. It is an operand on the stack, and the
  memory instruction will consume it to access memory.


### The offset

> This is the other half of the address to access. It is provided with the memory instruction.
  The final effective address is just the base address and the offset added together. These are  
  split into to parts to allow for compiler optimizations for certain operations. Once example 
  is copying a region of memory from one region to another: the base address walks along the 
  first region, with the offset being the distance to the copy.


### The alignment

> Provides a hint to the Wasm virtual machine on how the data is aligned in memory. Based on the 
  hardware, aligned memory is potentially faster to access, although the specification requires 
  that misaligned accesses must still succeed.

> A value of `x` for the alignment means that the data is aligned on a 2^x byte boundary.

> Common alignment values:

	- 0: 8 bit boundary
	- 1: 16 bit
	- 2: 32 bit
	- 3: 64 bit

> For random program generation, random values are interesting, because the memory operation 
  must succeed regardless of the alignment provided.

For a more in-depth write-up, this article is pretty good: https://rsms.me/wasm-intro, though
it is out of date compared to the current wasm instructions that include offset and alignment
as named instructions







