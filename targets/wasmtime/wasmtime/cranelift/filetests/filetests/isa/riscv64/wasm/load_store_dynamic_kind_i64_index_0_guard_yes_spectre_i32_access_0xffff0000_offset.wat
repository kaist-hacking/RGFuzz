;;! target = "riscv64"
;;!
;;! settings = ['enable_heap_access_spectre_mitigation=true']
;;!
;;! compile = true
;;!
;;! [globals.vmctx]
;;! type = "i64"
;;! vmctx = true
;;!
;;! [globals.heap_base]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 0, readonly = true }
;;!
;;! [globals.heap_bound]
;;! type = "i64"
;;! load = { base = "vmctx", offset = 8, readonly = true }
;;!
;;! [[heaps]]
;;! base = "heap_base"
;;! min_size = 0x10000
;;! offset_guard_size = 0
;;! index_type = "i64"
;;! style = { kind = "dynamic", bound = "heap_bound" }

;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
;; !!! GENERATED BY 'make-load-store-tests.sh' DO NOT EDIT !!!
;; !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!

(module
  (memory i64 1)

  (func (export "do_store") (param i64 i32)
    local.get 0
    local.get 1
    i32.store offset=0xffff0000)

  (func (export "do_load") (param i64) (result i32)
    local.get 0
    i32.load offset=0xffff0000))

;; function u0:0:
;; block0:
;;   lui a3,262140
;;   addi a5,a3,1
;;   slli a3,a5,2
;;   add a5,a0,a3
;;   trap_if heap_oob##(a5 ult a0)
;;   ld a3,8(a2)
;;   sltu a3,a3,a5
;;   ld a2,0(a2)
;;   add a0,a2,a0
;;   lui a5,65535
;;   slli a2,a5,4
;;   add a0,a0,a2
;;   sub a4,zero,a3
;;   not a2,a4
;;   and a2,a0,a2
;;   sw a1,0(a2)
;;   j label1
;; block1:
;;   ret
;;
;; function u0:1:
;; block0:
;;   lui a3,262140
;;   addi a5,a3,1
;;   slli a2,a5,2
;;   add a5,a0,a2
;;   trap_if heap_oob##(a5 ult a0)
;;   ld a2,8(a1)
;;   sltu a2,a2,a5
;;   ld a1,0(a1)
;;   add a0,a1,a0
;;   lui a5,65535
;;   slli a1,a5,4
;;   add a0,a0,a1
;;   sub a4,zero,a2
;;   not a1,a4
;;   and a2,a0,a1
;;   lw a0,0(a2)
;;   j label1
;; block1:
;;   ret