(module
  (import "env" "addToCrc" (func $addToCrc (param i32)))
  (memory $mem 1)
  (export "_memory" (memory $mem))
  (global $first (mut i32) (i32.const 0xDEADBEEF))
  (global $second (mut i32) (i32.const 0x0D15EA5E))
  (func $main (result i32)
    i32.const 0)
  (export "_main" (func $main))
  (func $crc_globals
    global.get $first
    call $addToCrc
    global.get $second
    call $addToCrc)
  (export "_crc_globals" (func $crc_globals)))
