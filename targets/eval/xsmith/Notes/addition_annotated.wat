;; --Header
(module
  (import "env" "__memory_base" (global (;0;) i32))
;; --End header
  (func (;0;) (export "_func") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
;; --Footer
;; The shorthand exports remove the need for counting the number of
;; functions, making the footer (mostly) independent from the rest of the code
  (func (;1;) (export "__post_instantiate") 
    global.get 0
    global.set 1
    global.get 1
    i32.const 5242880
    i32.add
    global.set 2)
  (global (;1;) (mut i32) (i32.const 0))
  (global (;2;) (mut i32) (i32.const 0)))
;; -- End footer
;; Using the shorthand notation in order to condense the footer
;;(export "_func" (func 0)))
