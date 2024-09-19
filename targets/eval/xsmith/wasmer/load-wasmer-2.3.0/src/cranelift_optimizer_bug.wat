(module
  (type (;0;) (func (result i32)))
  (type (;1;) (func (param i32) (result i32)))
  (type (;2;) (func))
  (func (;0;) (type 0) (result i32)
    (local i32 i64 f32 f64 i32 i32 i64 i64)
    i32.const 0
    call 2  ;; Stuff after the call is also important
    i32.const 2
    i32.const 1
    i32.const 0
    f64.load offset=37 align=4
    i32.const 655
    f64.load offset=40 align=4
    f64.add
    f32.demote_f64
    f32.store offset=77 align=2 ;; This store is important
    i32.const 1  ;; i32 and return is just used to return the function early
    return       ;; without taking care of the types on the stack
  )
  (func (;1;) (type 0) (result i32)
    unreachable  ;; Uncalled. Just didn't want to reorder function types.
  )
  (func (;2;) (type 1) (param i32) (result i32)
    (local i32 i64 f32 f64 i32 i64)
    i32.const 663 
    i32.const -2  ;; The negative for the value to be stored is important
    i32.store offset=36 align=1
    i32.const 0
  )
  (func (;3;) (type 2))
  (memory (;0;) 1)
  (global (;0;) (mut i32) i32.const 0)
  (global (;1;) (mut i32) i32.const -1)
  (export "_memory" (memory 0))
  (export "_main" (func 0))
  (export "_crc_globals" (func 3))
)
