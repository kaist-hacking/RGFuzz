(module
  (import "env" "addToCrc" (func $addToCrc (param i32)))
  (memory $mem 1)
  (export "_memory" (memory $mem))
  (func $main (result i32)
      i32.const 0
      i32.const 0xDEADBEEF
      i32.store offset=0 align=1
      i32.const 4
      i32.const 0x0D15EA5E
      i32.store offset=0 align=1
      i32.const 0)
  (export "_main" (func $main))
  (func $crc_globals)
  (export "_crc_globals" (func $crc_globals)))
