(module
  (memory 0)
  (func (export "grow") (param i32) (result i32) (memory.grow (local.get 0)))
)

(assert_return (invoke "grow" (i32.const 0)) (i32.const 0))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 0))
(assert_return (invoke "grow" (i32.const 0)) (i32.const 1))
(assert_return (invoke "grow" (i32.const 2)) (i32.const 1))
(assert_return (invoke "grow" (i32.const 800)) (i32.const 3))
(assert_return (invoke "grow" (i32.const 0x10000)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 64736)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 803))

(module
  (memory 0 10)
  (func (export "grow") (param i32) (result i32) (memory.grow (local.get 0)))
)

(assert_return (invoke "grow" (i32.const 0)) (i32.const 0))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 0))
(assert_return (invoke "grow" (i32.const 1)) (i32.const 1))
(assert_return (invoke "grow" (i32.const 2)) (i32.const 2))
(assert_return (invoke "grow" (i32.const 6)) (i32.const 4))
(assert_return (invoke "grow" (i32.const 0)) (i32.const 10))
(assert_return (invoke "grow" (i32.const 1)) (i32.const -1))
(assert_return (invoke "grow" (i32.const 0x10000)) (i32.const -1))

;; As the argument of control constructs and instructions

(module
  (memory 1)

  (func (export "as-br-value") (result i32)
    (block (result i32) (br 0 (memory.grow (i32.const 0))))
  )

  (func (export "as-br_if-cond")
    (block (br_if 0 (memory.grow (i32.const 0))))
  )
  (func (export "as-br_if-value") (result i32)
    (block (result i32)
      (drop (br_if 0 (memory.grow (i32.const 0)) (i32.const 1))) (i32.const 7)
    )
  )
  (func (export "as-br_if-value-cond") (result i32)
    (block (result i32)
      (drop (br_if 0 (i32.const 6) (memory.grow (i32.const 0)))) (i32.const 7)
    )
  )

  (func (export "as-br_table-index")
    (block (br_table 0 0 0 (memory.grow (i32.const 0))))
  )
  (func (export "as-br_table-value") (result i32)
    (block (result i32)
      (br_table 0 0 0 (memory.grow (i32.const 0)) (i32.const 1)) (i32.const 7)
    )
  )
  (func (export "as-br_table-value-index") (result i32)
    (block (result i32)
      (br_table 0 0 (i32.const 6) (memory.grow (i32.const 0))) (i32.const 7)
    )
  )

  (func (export "as-return-value") (result i32)
    (return (memory.grow (i32.const 0)))
  )

  (func (export "as-if-cond") (result i32)
    (if (result i32) (memory.grow (i32.const 0))
      (then (i32.const 0)) (else (i32.const 1))
    )
  )
  (func (export "as-if-then") (result i32)
    (if (result i32) (i32.const 1)
      (then (memory.grow (i32.const 0))) (else (i32.const 0))
    )
  )
  (func (export "as-if-else") (result i32)
    (if (result i32) (i32.const 0)
      (then (i32.const 0)) (else (memory.grow (i32.const 0)))
    )
  )

  (func (export "as-select-first") (param i32 i32) (result i32)
    (select (memory.grow (i32.const 0)) (local.get 0) (local.get 1))
  )
  (func (export "as-select-second") (param i32 i32) (result i32)
    (select (local.get 0) (memory.grow (i32.const 0)) (local.get 1))
  )
  (func (export "as-select-cond") (result i32)
    (select (i32.const 0) (i32.const 1) (memory.grow (i32.const 0)))
  )

  (func $f (param i32 i32 i32) (result i32) (i32.const -1))
  (func (export "as-call-first") (result i32)
    (call $f (memory.grow (i32.const 0)) (i32.const 2) (i32.const 3))
  )
  (func (export "as-call-mid") (result i32)
    (call $f (i32.const 1) (memory.grow (i32.const 0)) (i32.const 3))
  )
  (func (export "as-call-last") (result i32)
    (call $f (i32.const 1) (i32.const 2) (memory.grow (i32.const 0)))
  )

  (type $sig (func (param i32 i32 i32) (result i32)))
  (table funcref (elem $f))
  (func (export "as-call_indirect-first") (result i32)
    (call_indirect (type $sig)
      (memory.grow (i32.const 0)) (i32.const 2) (i32.const 3) (i32.const 0)
    )
  )
  (func (export "as-call_indirect-mid") (result i32)
    (call_indirect (type $sig)
      (i32.const 1) (memory.grow (i32.const 0)) (i32.const 3) (i32.const 0)
    )
  )
  (func (export "as-call_indirect-last") (result i32)
    (call_indirect (type $sig)
      (i32.const 1) (i32.const 2) (memory.grow (i32.const 0)) (i32.const 0)
    )
  )
  (func (export "as-call_indirect-index") (result i32)
    (call_indirect (type $sig)
      (i32.const 1) (i32.const 2) (i32.const 3) (memory.grow (i32.const 0))
    )
  )

  (func (export "as-local.set-value") (local i32)
    (local.set 0 (memory.grow (i32.const 0)))
  )
  (func (export "as-local.tee-value") (result i32) (local i32)
    (local.tee 0 (memory.grow (i32.const 0)))
  )
  (global $g (mut i32) (i32.const 0))
  (func (export "as-global.set-value") (local i32)
    (global.set $g (memory.grow (i32.const 0)))
  )

  (func (export "as-unary-operand") (result i32)
    (i32.clz (memory.grow (i32.const 0)))
  )

  (func (export "as-binary-left") (result i32)
    (i32.add (memory.grow (i32.const 0)) (i32.const 10))
  )
  (func (export "as-binary-right") (result i32)
    (i32.sub (i32.const 10) (memory.grow (i32.const 0)))
  )

  (func (export "as-test-operand") (result i32)
    (i32.eqz (memory.grow (i32.const 0)))
  )

  (func (export "as-compare-left") (result i32)
    (i32.le_s (memory.grow (i32.const 0)) (i32.const 10))
  )
  (func (export "as-compare-right") (result i32)
    (i32.ne (i32.const 10) (memory.grow (i32.const 0)))
  )

  (func (export "as-memory.grow-size") (result i32)
    (memory.grow (memory.grow (i32.const 0)))
  )
)

(assert_return (invoke "as-br-value") (i32.const 1))

(assert_return (invoke "as-br_if-cond"))
(assert_return (invoke "as-br_if-value") (i32.const 1))
(assert_return (invoke "as-br_if-value-cond") (i32.const 6))

(assert_return (invoke "as-br_table-index"))
(assert_return (invoke "as-br_table-value") (i32.const 1))
(assert_return (invoke "as-br_table-value-index") (i32.const 6))

(assert_return (invoke "as-return-value") (i32.const 1))

(assert_return (invoke "as-if-cond") (i32.const 0))
(assert_return (invoke "as-if-then") (i32.const 1))
(assert_return (invoke "as-if-else") (i32.const 1))

(assert_return (invoke "as-select-first" (i32.const 0) (i32.const 1)) (i32.const 1))
(assert_return (invoke "as-select-second" (i32.const 0) (i32.const 0)) (i32.const 1))
(assert_return (invoke "as-select-cond") (i32.const 0))

(assert_return (invoke "as-call-first") (i32.const -1))
(assert_return (invoke "as-call-mid") (i32.const -1))
(assert_return (invoke "as-call-last") (i32.const -1))

(assert_return (invoke "as-call_indirect-first") (i32.const -1))
(assert_return (invoke "as-call_indirect-mid") (i32.const -1))
(assert_return (invoke "as-call_indirect-last") (i32.const -1))
(assert_trap (invoke "as-call_indirect-index") "undefined element")

(assert_return (invoke "as-local.set-value"))
(assert_return (invoke "as-local.tee-value") (i32.const 1))
(assert_return (invoke "as-global.set-value"))

(assert_return (invoke "as-unary-operand") (i32.const 31))

(assert_return (invoke "as-binary-left") (i32.const 11))
(assert_return (invoke "as-binary-right") (i32.const 9))

(assert_return (invoke "as-test-operand") (i32.const 0))

(assert_return (invoke "as-compare-left") (i32.const 1))
(assert_return (invoke "as-compare-right") (i32.const 1))

(assert_return (invoke "as-memory.grow-size") (i32.const 1))
