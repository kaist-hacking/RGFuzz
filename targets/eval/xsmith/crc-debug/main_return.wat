(module
  (import "env" "addToCrc" (func $addToCrc (param i32)))
  (memory $mem 1)
  (export "_memory" (memory $mem))
  (func $main (result i32)
    i32.const 0xDEADBEEF)
  (export "_main" (func $main))
  (func $crc_globals)
  (export "_crc_globals" (func $crc_globals))
  (type (;0;) (func (param i32)))
  (type (;1;) (func (result i32)))
  (type (;2;) (func (param i32) (result i32)))
  (type (;3;) (func)))
