;;! target = "x86_64"

(module
    (func (result f64)
        i32.const 1
        f64.convert_i32_s
        block
        end
    )
)
;;      	 55                   	push	rbp
;;      	 4889e5               	mov	rbp, rsp
;;      	 4883ec08             	sub	rsp, 8
;;      	 4d8b5e08             	mov	r11, qword ptr [r14 + 8]
;;      	 4d8b1b               	mov	r11, qword ptr [r11]
;;      	 4939e3               	cmp	r11, rsp
;;      	 0f8725000000         	ja	0x3d
;;   18:	 4c893424             	mov	qword ptr [rsp], r14
;;      	 b801000000           	mov	eax, 1
;;      	 f20f2ac0             	cvtsi2sd	xmm0, eax
;;      	 4883ec08             	sub	rsp, 8
;;      	 f20f110424           	movsd	qword ptr [rsp], xmm0
;;      	 f20f100424           	movsd	xmm0, qword ptr [rsp]
;;      	 4883c408             	add	rsp, 8
;;      	 4883c408             	add	rsp, 8
;;      	 5d                   	pop	rbp
;;      	 c3                   	ret	
;;   3d:	 0f0b                 	ud2	
