;; wasmer loop_test.wat -i _start

(module
  (func (;0;) (result f32)
    ;; Need to declare this in every function
    (local $counter i32)

    ;; counter setup
    i32.const 10
    local.set $counter

    ;;initial parameter
    f32.const 19

    ;;start printing the loop
    loop (param f32) (result f32)
      ;;binop with the left being a dummy (parameter) and the right being any expr chain
      f32.const 2
      f32.add

      ;;Decrement the counter
      local.get $counter
      i32.const -1
      i32.add
      local.tee $counter ;; Don't forget to save the counter back

      ;;At this point, the conditional and the parameter value are on the stack
      ;;  If the branch is taken, it consumes the paramter for the start of the loop
      ;;  If not, the parameter is used as the result type for the loop
      br_if 0
    end 
  )
  (export "_start" (func 0)))
