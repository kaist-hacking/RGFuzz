;;! target = "x86_64"

(module
    (func (result i64)
        (local $foo i64)  
        (local $bar i64)

        (i64.const 1)
        (local.set $foo)

        (i64.const 2)
        (local.set $bar)

        (local.get $foo)
        (local.get $bar)
        (i64.shr_u)
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec18             	sub	rsp, 0x18
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f873c000000         	ja	0x54
;;   18:	 4531db               	xor	r11d, r11d
;;      	 4c895c2410           	mov	qword ptr [rsp + 0x10], r11
;;      	 4c895c2408           	mov	qword ptr [rsp + 8], r11
;;      	 4c893424             	mov	qword ptr [rsp], r14
;;      	 48c7c001000000       	mov	rax, 1
;;      	 4889442410           	mov	qword ptr [rsp + 0x10], rax
;;      	 48c7c002000000       	mov	rax, 2
;;      	 4889442408           	mov	qword ptr [rsp + 8], rax
;;      	 488b4c2408           	mov	rcx, qword ptr [rsp + 8]
;;      	 488b442410           	mov	rax, qword ptr [rsp + 0x10]
;;      	 48d3e8               	shr	rax, cl
;;      	 4883c418             	add	rsp, 0x18
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   54:	 0f0b                 	ud2	