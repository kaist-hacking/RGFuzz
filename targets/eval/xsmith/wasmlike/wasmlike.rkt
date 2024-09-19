#lang clotho
;; -*- mode: Racket -*-
;;
;; Copyright (c) 2023 The University of Utah
;; All rights reserved.
;;
;; Redistribution and use in source and binary forms, with or without
;; modification, are permitted provided that the following conditions are met:
;;
;;   * Redistributions of source code must retain the above copyright notice,
;;     this list of conditions and the following disclaimer.
;;
;;   * Redistributions in binary form must reproduce the above copyright
;;     notice, this list of conditions and the following disclaimer in the
;;     documentation and/or other materials provided with the distribution.
;;
;; THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
;; AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
;; IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
;; ARE DISCLAIMED.  IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE
;; LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
;; CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
;; SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
;; INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
;; CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
;; ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
;; POSSIBILITY OF SUCH DAMAGE.

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

(require
  (only-in racket/base [append r:append])
  (for-syntax racket/base syntax/parse)
  xsmith
  xsmith/racr-convenience
  xsmith/app
  racr
  racket/system
  racket/port
  racket/class
  racket/pretty
  racket/string
  racket/match
  racket/set
  racket/dict
  racket/hash
  racket/path
  (except-in racket/list empty))

(define wasmlike-version 1.0)
 
(define-spec-component wasmlike)

(define function-def-falloff-param
  (make-parameter 10))
(define debug-show-s-exp-param
  (make-parameter #f))

(define true-strings  '("true"  "t" "#t" "yes" "y"))
(define false-strings '("false" "f" "#f" "no"  "n"))
(define (string->bool bool-string)
  (define s (string-downcase bool-string))
  (cond [(member s true-strings) #t]
        [(member s false-strings) #f]
        [else (error
               'string->bool
               (string-append
                "While parsing argument for ~a,\n"
                "expected “true”, “t”, “#t”, “yes”, “y”, “false”, “f”, “#f”, “no”, or “n”.\n"
                "Got ~a.\n")
               bool-string)]))

(define (choose-random-sign) (< (random) 0.5))
;;todo use a passed in node to assign sign distributions per type instead of blanket-wise

(define (choose-random-float) (+ (random -1000 1000) (random)))

(define-syntax (append stx)
  (syntax-parse stx
    [(_ arg ...)
     #`(with-handlers ([(λ (e) #t) (λ (e) (eprintf "bad append at line ~v\n" 
                                                   #,(syntax-line stx)) 
                                          (raise e))])
         (r:append arg ...))]))

(add-property
 wasmlike
 choice-weight
 ;;Default
 [#f (λ (n) 10)]

 ;; Indentation indicates a sub-type of node
 ;; Abstract nodes can have properties which are inherited by their subclasses, but in this 
 ;; case each subclass defines its own property that overrides the property of the
 ;; abstract class, so it is never actually used.
 ;; ! The inner weight is the only one used

 [Call 30]
 [IndirectCall 30]
 [VariableExpr 20]
   ;[VariableGet 10]
   ;[VariableSet 10]
   ;[VariableTee 10]
 [Literal 5]
   ;[LiteralIntThirtyTwo 10]
   ;[LiteralIntSixtyFour 10]
   ;[LiteralFloatThirtyTwo 10]
   ;[LiteralFloatSixtyFour 10]
 [Binop 25] 
   ;[Addition 10]
   ;[Subtraction 10]
   ;[Multiplication 10]
   ;[Division 10]
   ;[Remainder  10]
   ;[And 10]
   ;[Or 10]
   ;[Xor 10]
   ;[ShiftLeft 10]
   ;[ShiftRight 10]
   ;[RotateLeft 10]
   ;[RotateRight  10]
   ;[Min 10]
   ;[Max 10]
   ;[CopySign 10]
 [Comparison 10]
   ;[Equal 10]
   ;[NotEqual 10]
   ;[LessThan 10]
   ;[GreaterThan 10]
   ;[LessThanOrEqual 10]
   ;[GreaterThanOrEqual 10]
 [Unop 20]
   ;[CountLeadingZero 10]
   ;[CountTrailingZero 10]
   ;[NonZeroBits 10]
   ;[AbsoluteValue 10]
   ;[Negate 10]
   ;[SquareRoot 10]
   ;[Ceiling 10]
   ;[Floor 10]
   ;[Truncate 10]
   ;[Nearest 10]
 [Testop 10]
   ;[EqualZero 10]
 [Noop 10]
 [Select 10]
 [NestingExpr 15]
   ;[IfElse 10]
   ;[Block 10]
   ;[IdiomaticLoop 10]
   ;--[Loop 10]
 [BranchExpr 10]
   ;[Branch 10]
   ;[BranchIf 10]
 [MemStore 15] 
   ;[StoreIntThirtyTwo 10]
   ;[StoreFloatThirtyTwo 10]
   ;[StoreIntSixtyFour 10]
   ;[StoreFloatSixtyFour  10]
   ;[StoreEight 10]
   ;[StoreSixteen 10]
   ;[StoreThirtyTwo 10]
 [MemLoad 15]
   ;[LoadIntThirtyTwo 10]
   ;[LoadFloatThirtyTwo 10]
   ;[LoadIntSixtyFour 10]
   ;[LoadFloatSixtyFour 10]
   ;[LoadEightSigned 10]
   ;[LoadEightUnsigned 10]
   ;[LoadSixteenSigned 10]
   ;[LoadSixteenUnsigned 10]
   ;[LoadThirtyTwoSigned 10]
   ;[LoadThirtyTwoUnsigned 10]
 [TypeConversion 15] 
   ;[TruncateFloat 10]
   ;[ConvertInt 10]
   ;[Wrap 10]
   ;[Extend 10]
   ;[SignExtendThirtyTwo 10]
   ;[SignExtendSixtyFour 10]
   ;[Demote 10]
   ;[Promote 10]
   ;[ReinterpretIntThirtyTwo 10]
   ;[ReinterpretIntSixtyFour 10]
   ;[ReinterpretFloatThirtyTwo 10]
   ;[ReinterpretFloatSixtyFour 10]
)

;;Other weights
;; It's important to note that ints don't care about signed or unsigned, just the operation in
;; question does. They just happen to be a special number that represents a particular bit pattern
(define max-unsigned-int-weight 0.0015) ;; 0.15%
(define max-signed-int-weight 0.0015)   ;; 0.15%
(define min-signed-int-weight 0.0015)   ;; 0.15%

(define new-function-probability 0.50)  ;; 50%


;; This defines the layout of the grammar.
(add-to-grammar
 wasmlike
 [Program #f ([globals : VariableDef *]
              [functions : Func *]
              [main : Func])]
 [Func #f ([params : Param *]
           [locals : VariableDef *]
           [memorydefs : MemStore *]
           [root : Expr]
           [name]
           [type])
       #:prop wont-over-deepen #t
       #:prop binder-info (#:binder-style definition)]
 [Param #f ([type]
            [name])
        #:prop binder-info (#:binder-style definition)]
 [Call Expr ([function : FunctionReference]
             [argnode : Arguments])]
 [IndirectCall Expr ([function : FunctionReference]
              [argnode : Arguments])]
 [FunctionReference #f ([name])
                    #:prop reference-info (read)]
;  [IndirectFunctionReference #f ([name])
;                     #:prop reference-info (read)]
 [Arguments #f ([args : Expr *])]
 [VariableExpr Expr ()
               #:prop may-be-generated #f]
 [VariableDef #f ([init : Literal] ;;only used in globals
                  [type]
                  [name])
              #:prop binder-info (#:binder-style definition)]
 [VariableGet VariableExpr ([name])
              #:prop reference-info (read)]
 [VariableSet VariableExpr ([val : Expr]
                    [name]
                    [expr : Expr])
              #:prop reference-info (read)]
 [VariableTee VariableExpr ([val : Expr]
                    [name])
              #:prop reference-info (read)]
 [Expr #f ()
       #:prop may-be-generated #f]
 [Literal Expr ()
          #:prop may-be-generated #f]
 [DummyLiteral Literal () ;; Dummy that should not be printed: useful for loop parameters, since parameters
               #:prop may-be-generated #f] ;; are popped outside, then pushed on the stack inside
 [NestingExpr Expr()
            #:prop may-be-generated #f]
 [BranchExpr Expr()
             #:prop may-be-generated #f]

 ;;todo - make a list of probabilities and list
 ;; 32 bit maxes and mins fit into 64 bit
 ;; what about max32 + 1?
 [LiteralIntThirtyTwo Literal ([v = (let* ([choice (random)]
                                           [max-unsigned-int max-unsigned-int-weight]
                                           [max-signed-int (+ max-unsigned-int max-signed-int-weight)]
                                           [min-signed-int (+ max-signed-int min-signed-int-weight)])
                                      ;;(printf "Choice: ~a    max_u_int: ~a   max_s_int: ~a   min_s_int: ~a\n"
                                      ;;        choice max-unsigned-int max-signed-int min-signed-int)

                                      (cond [(< choice max-unsigned-int) #xFFFFFFFF]
                                            [(< choice max-signed-int) #x7FFFFFFF]
                                            [(< choice min-signed-int) #x80000000]
                                            [else (random -1000 1000)]))])]


 [LiteralIntSixtyFour Literal ([v = (let* ([choice (random)]
                                           [max-unsigned-int max-unsigned-int-weight]
                                           [max-signed-int (+ max-unsigned-int max-signed-int-weight)]
                                           [min-signed-int (+ max-signed-int min-signed-int-weight)])
                                      (cond [(< choice max-unsigned-int) #xFFFFFFFFFFFFFFFF]
                                            [(< choice max-signed-int) #x7FFFFFFFFFFFFFFF]
                                            [(< choice min-signed-int) #x8000000000000000]
                                            [else (random -1000 1000)]))])]
 [LiteralFloatThirtyTwo Literal ([v = (choose-random-float)])]
 [LiteralFloatSixtyFour Literal ([v = (choose-random-float)])]
 [Noop Expr ([expr : Expr])]
 [Binop Expr ([l : Expr] [r : Expr]) ;;Binops take a left and a right and produce the same type
        #:prop may-be-generated #f]  ;; as the result
 [Comparison Expr ([l : Expr]
                   [r : Expr]
                   [sign = (choose-random-sign)]) ;;Comparisons take a left and a right of the same type
             #:prop may-be-generated #f]  ;; but return a boolean, which is always an i32
                                          ;; Section 2.4.1 in the spec has more detail
 [Unop Expr ([expr : Expr]) ;;Unops take one operand and return a value of that type 
       #:prop may-be-generated #f]
 [Testop Expr ([expr : Expr]) ;;Testops take one operand but return a boolean (i32)
         #:prop may-be-generated #f]

 [Addition Binop ()]
 [Subtraction Binop ()]
 [Multiplication Binop ()]
 [Division Binop ([sign = (choose-random-sign)])] ;;These signs only matter for ints, not floats
 [Remainder Binop ([sign = (choose-random-sign)])]
 [And Binop ()]
 [Or Binop ()]
 [Xor Binop ()]
 [ShiftLeft Binop ()]
 [ShiftRight Binop ([sign = (choose-random-sign)])]
 [RotateLeft Binop ()]
 [RotateRight Binop ()]
 [Min Binop ()]
 [Max Binop ()]
 [CopySign Binop ()]

 [Equal Comparison ()]
 [NotEqual Comparison ()]
 [LessThan Comparison ()]
 [GreaterThan Comparison ()]
 [LessThanOrEqual Comparison ()]
 [GreaterThanOrEqual Comparison ()]

 [CountLeadingZero Unop ()]
 [CountTrailingZero Unop ()]
 [NonZeroBits Unop ()]
 [AbsoluteValue Unop ()]
 [Negate Unop ()]
 [SquareRoot Unop ()]
 [Ceiling Unop ()]
 [Floor Unop ()]
 [Truncate Unop ()]
 [Nearest Unop ()]

 [EqualZero Testop ()]

 [Select Expr ([l : Expr]
               [r : Expr]
               [selector : Expr])]
 
 [IfElse NestingExpr ([cond : Expr]
                      [then : Expr]
                      [else : Expr])]
 [Block NestingExpr ([expr : Expr])]
 [IdiomaticLoop NestingExpr ([iterations = (random 10 200)]
                             [initparam : Expr]
                             [body : Expr]
                             [name])]

 ;[Loop NestingExpr ([paramexprs : Expr *]
 ;                   [paramtypes]
 ;                   [looptype]
 ;                   [body : Expr])]
 [Branch BranchExpr ([vals : Expr *] 
                     [targettypes] ;;product-type of target types (either a return-type or the param types)
                     [targetindex])] ;; a number depth
 [BranchIf BranchExpr ([cond : Expr] 
                       [vals : Expr *] ;;used for target: typed to match target
                       [expr : Expr];;used for parent: typed to fit the ast hole
                       [targettypes]
                       [targetindex])]
 [MemStore Expr ([address = (random 0 800)] ;; Page size is 65536. Because of 64 bit values, the 
                 [value : Expr]             ;;   largest address we want to generate is 65534.
                 [offset = (random 0 100)]  ;;   We can add a factor of safety with the static offset: offset can't be larger than 100
                 ;;[alignment = (random 3)] ;; Alignment is 2^a, where a is this number, and can't exceed the number of bytes of the operation.
                                            ;;   For normal stores (any type), we reduce the alignment to the maximum allowed
                                            ;;   during rendering. For a quick reference, 6.5.5 has subscript numbers like this: memarg_2
                                            ;;   The maximum alignment is 2^a = subscript
                                            ;; Alignment is handled below after breaking out memory stores each into their own type
                 [expr : Expr])            
           #:prop may-be-generated #f]
 [StoreIntThirtyTwo MemStore ([alignment = (random 2)])] 
 [StoreFloatThirtyTwo MemStore ([alignment = (random 2)])] 
 [StoreIntSixtyFour MemStore ([alignment = (random 3)])] 
 [StoreFloatSixtyFour MemStore ([alignment = (random 3)])] 
 [StoreEight MemStore ([alignment = 0])]
 [StoreSixteen MemStore ([alignment = (random 1)])]
 [StoreThirtyTwo MemStore ([alignment = (random 2)])]

 [MemLoad Expr ([address = (random 0 800)] ;; TODO - find way to generate this from a distibution, or abstract variables on top of this.
                [offset = (random 0 100)])
                ;;[alignment = (random 3)])
          #:prop may-be-generated #f]
 [LoadIntThirtyTwo MemLoad ([alignment = (random 2)])] 
 [LoadFloatThirtyTwo MemLoad ([alignment = (random 2)])] 
 [LoadIntSixtyFour MemLoad ([alignment = (random 3)])] 
 [LoadFloatSixtyFour MemLoad ([alignment = (random 3)])] 
 [LoadEightSigned MemLoad ([alignment = 0])]
 [LoadEightUnsigned MemLoad ([alignment = 0])]
 [LoadSixteenSigned MemLoad ([alignment = (random 1)])]
 [LoadSixteenUnsigned MemLoad ([alignment = (random 1)])]
 [LoadThirtyTwoSigned MemLoad ([alignment = (random 2)])]
 [LoadThirtyTwoUnsigned MemLoad ([alignment = (random 2)])]

 ;; Type conversions
 [TypeConversion Expr ([expr : Expr])
                 #:prop may-be-generated #f]
 [TruncateFloat TypeConversion ([sign = (choose-random-sign)])]  ;; float -> int
 [ConvertInt TypeConversion ([sign = (choose-random-sign)])]     ;; int -> float
 [Wrap TypeConversion ()]                                        ;; i64 -> i32
 [Extend TypeConversion ([sign = (choose-random-sign)])]         ;; i32 -> i64
 [SignExtendThirtyTwo TypeConversion ([width = (list-ref (list "16" "8") (random 2))])]
 [SignExtendSixtyFour TypeConversion ([width = (list-ref (list "32" "16" "8") (random 3))])]

 [Demote TypeConversion ()]                                      ;; f64 -> f32
 [Promote TypeConversion ()]                                     ;; f32 -> f64
 [ReinterpretIntThirtyTwo TypeConversion ()]
 [ReinterpretIntSixtyFour TypeConversion ()]
 [ReinterpretFloatThirtyTwo TypeConversion ()]
 [ReinterpretFloatSixtyFour TypeConversion ()]
)

(add-property
  wasmlike
  feature
  ;; floating point is by default on
  ;; Turn off type conversions that have anything to do with floats
  [LiteralFloatThirtyTwo floating-point]
  [LiteralFloatSixtyFour floating-point]
  [TruncateFloat floating-point]
  [ConvertInt floating-point]
  [Demote floating-point]
  [Promote floating-point]
  [ReinterpretIntThirtyTwo floating-point]
  [ReinterpretIntSixtyFour floating-point]
  [ReinterpretFloatThirtyTwo floating-point]
  [ReinterpretFloatSixtyFour floating-point]
  ;; Turn off operators that work with floats
  [Min floating-point]
  [Max floating-point]
  [CopySign floating-point]
  [AbsoluteValue floating-point]
  [Negate floating-point]
  [SquareRoot floating-point]
  [Ceiling floating-point]
  [Floor floating-point]
  [Truncate floating-point]
  [Nearest floating-point]
  
  ;; Turn off sign-extension operators
  [SignExtendThirtyTwo sign-extension]
  [SignExtendSixtyFour sign-extension]

  ;; Turn off indirect calls
  [IndirectCall indirect-calls]
  
  ;; Turn off non-trapping float-to-int conversion operators
  [TruncateFloat non-trapping-float-to-int])


;; gathers all IndirectCall nodes in a list
(add-attribute
  wasmlike
  table-funcs
  [Program (λ (n) (att-value 'xsmith_find-descendants n (λ (d) (equal? (node-type d) 'IndirectCall))))])


(add-property
  wasmlike
  lift-type->ast-binder-type
  [#f (λ (type) (begin
                   (if (function-type? type)
                     'Func
                     'VariableDef)))])

(define (get-type-conversion-symbol parent-type child-type)
  (begin
    (printf "Parent type: ~a, child type: ~a\n" parent-type child-type)
    (cond [(equal? parent-type child-type) 
           ;;(printf "Type conversion chosen: 'Noop\n")
           'Noop]
          [(and (or (equal? parent-type i32) (equal? parent-type i64))
                (or (equal? child-type f32) (equal? child-type f64)))
           ;;(printf "Type conversion chosen: 'TruncateFloat\n")
           'TruncateFloat]
          [(and (or (equal? parent-type f32) (equal? parent-type f64))
                (or (equal? child-type i32) (equal? child-type i64)))
           ;;(printf "Type conversion chosen: 'ConvertInt\n")
           'ConvertInt]
          [(and (equal? parent-type i32)
                (equal? child-type i64))
           ;;(printf "Type conversion chosen: 'Wrap\n")
           'Wrap]
          [(and (equal? parent-type i64)
                (equal? child-type i32))
           ;;(printf "Type conversion chosen: 'Extend\n")
           'Extend]
          [(and (equal? parent-type f32)
                (equal? child-type f64))
           ;;(printf "Type conversion chosen: Demote\n")
           'Demote]
          [(and (equal? parent-type f64)
                (equal? child-type f32))
           ;;(printf "Type conversion chosen: Promote\n")
           'Promote]
          [else (printf "!!! [get-type-conversion-symbol] Getting here should not be possible !!!\n")])))
          ;; purposefully return void and crash

(add-property
  wasmlike
  fresh

  ;; Constrain the main function
  [Func 
    (if (and (ast-has-parent? (current-hole))  ;; Check for the existence of a parent /before/ asking the parent it's type.
             (equal? (ast-child 'main (ast-parent (current-hole))) (current-hole)))
      ;; main function: no params, special name
      (hash 'params (list)
            'name "main"
            'type (function-type (product-type (list)) i32))
      ;; new function: the number of parameters must match the number given
      ;;  in the type of the node (from the lifting)
      ;; Force type exploration
      (let ([type (concretize-type (att-value 'xsmith_type (current-hole)))])
        (hash 'params (map (λ (t) 
                              (make-fresh-node 'Param (hash 'type t)))
                           (product-type-inner-type-list!
                             (function-type-arg-type! type))))))]
  [Arguments
    (let*
      ([call-node (parent-node (current-hole))]
       [target-function (binding-ast-node (att-value 'xsmith_binding (ast-child 'function call-node)))]
       [target-params (ast-children (ast-child 'params target-function))])
      (begin
        (hash 'args (length target-params))))]


  [Param (hash 'name (fresh-var-name "param_"))] ;; Name generated parameters, since they are not
                                                 ;; generated by the lifting machinery
  [IdiomaticLoop (hash 'body (make-fresh-node ;; todo - get a fresh local variable each time
                               'Addition  ;; todo - is it possible to specify a binop here instead of add?
                               (hash 'l (make-fresh-node 'DummyLiteral)))
                       'name (fresh-var-name "loop_"))]
  ;;'r (make-hole 'Expr))))]


  ;; Notes for allowing params in loops/blocks
  ; - Generate n params
  ; - Generate some instructions for the loop
  ; - Must generate binops until params exhausted (params are put on the stack and must be used immediately)
  ;   - First binop must fit the result type, so all params will need to be typeshifted
  ;   - Left child is each param in order **already on the stack**. Don't re-generate these. Instead, use dummy literals
  ;     - For each parameter, add a type-shift if need (trunc_sat or convert). Apply conversions during printing
  ;   - Right child the next binop
  ;; Binop chain should look like this:
  ;             loop
  ;               |
  ;               T (type conversion)
  ;               |
  ;               + (binop)
  ;              / \
  ;   (param)  (1)  T
  ;                 |
  ;                 +
  ;                / \
  ;              (2)  T
  ;                   |
  ;                   E  (Expr)
  ;[Loop (let* ;; Binop chain method
  ;        ([num-params 2] ;;(random 0 4)]
  ;         [param-exprs (for/list ([i (in-range num-params)])
  ;                        (make-hole 'Expr))]
  ;         ;; Concretize the types of the parameters here so that we can choose the correct type 
  ;         ;; conversion to string stuff together. If we don't, we have to use a hole, and we can't
  ;         ;; specify the children of a hole
  ;         [param-types (for/list ([p param-exprs])
  ;                        (concretize-type (fresh-type-variable)))]
  ;         [loop-type (concretize-type (att-value 'xsmith_type (current-hole)))];;just choose some random types and save them
  ;         [_ (unify! loop-type (att-value 'xsmith_type (current-hole)))]
  ;         [empty-expr (make-hole 'Expr)]
  ;         [binop-chain (make-fresh-node
  ;                        (get-type-conversion-symbol
  ;                          loop-type
  ;                          (list-ref param-types 0))
  ;                        (hash 'expr (for/fold ([subtree empty-expr])
  ;                                      ([param-type (reverse param-types)])
  ;                                      (make-fresh-node 
  ;                                        'Addition
  ;                                        (hash 'l (make-fresh-node
  ;                                                      'DummyLiteral)
  ;                                              'r (make-fresh-node
  ;                                                       (get-type-conversion-symbol
  ;                                                         param-type
  ;                                                         (concretize-type (att-value 'xsmith_type subtree)))
  ;                                                       (hash 'expr subtree)))))))])
  ;         ;[loop-body (make-fresh-node
  ;         ;             (get-type-conversion-symbol
  ;         ;               loop-type
  ;         ;               (list-ref param-types 0)) ;;this is the type at the top of the binop chain
  ;         ;             (hash 'expr binop-chain))])
  ;        (hash 'paramtypes param-types
  ;              'paramexprs param-exprs
  ;              'looptype loop-type
  ;              'body binop-chain))]

  [Branch (if (ast-has-parent? (current-hole)) ;; Only check for targets if we are attached to the tree
                                             ;; For example: in generating a branch instruction for the
                                             ;; for loop below while not connected to the tree, we
                                             ;; must manually specify the target
            (match-let ([(cons index node) (choose-br-target (current-hole))])
              (define vals-type (product-type #f))
              (define target-type (if (equal? (node-type node) 'Func)
                                    (product-type (list (function-type-return-type!
                                                          (att-value 'xsmith_type node))))
                                    (if (equal? (node-type node) 'Loop) ;; branch must produce params for loop
                                      (product-type (map (λ (p)
                                                            (att-value 'xsmith_type p))
                                                         (ast-children (ast-child 'params node))))
                                      ;;Everything else takes a single parameter of their node-type (including idiomatic loops)
                                      (product-type (list (att-value 'xsmith_type node))))))
              (unify! vals-type target-type)
              ;; Set up the proper number of vals based on the target-type 
              (define vals (map (λ (vt)
                                   (make-hole 'Expr))
                                (product-type-inner-type-list! vals-type)))
              (hash 'targetindex index
                    'targettypes target-type
                    'vals vals))
            (hash))]
  [BranchIf (if (ast-has-parent? (current-hole))
              (match-let ([(cons index node) (choose-br-target (current-hole))])



                (define vals-type (product-type #f))
                (define target-type (if (equal? (node-type node) 'Func)
                                      (product-type (list (function-type-return-type!
                                                            (att-value 'xsmith_type node))))
                                      (if (equal? (node-type node) 'Loop) ;; branch must produce params for loop
                                        (product-type (map (λ (p)
                                                              (att-value 'xsmith_type p))
                                                           (ast-children (ast-child 'params node))))
                                        ;;Everything else takes a single parameter of their node-type
                                        (product-type (list (att-value 'xsmith_type node))))))
                (unify! vals-type target-type)
                ;; Set up the proper number of vals based on the target-type 
                (define vals (map (λ (vt)
                                     (make-hole 'Expr))
                                  (product-type-inner-type-list! vals-type)))
                (hash 'targetindex index
                      'targettypes target-type
                      'vals vals))
              (hash))]

  )

(add-property
  wasmlike
  depth-increase
  ;; The depth of a function relies on its lift depth: something that is really hard to determine at
  ;; the function node itself, since at that point, it's only a RACR node, not a full xsmith node
  ;; with binding information. Instead we can target the call node, or more accurately, the function
  ;; reference child. If we reset the depth here, we should see an impact on the generated function.
  ;; If the function was already created, we don't affect any children down the tree
  [FunctionReference (λ (n)
                        (let* ([functions-node (ast-child 'functions (top-ancestor-node n))]
                               [num-functions (length (ast-children functions-node))]
                               [current-depth (+ 1 (att-value 'xsmith_ast-depth (parent-node n)))] 
                               [number-past (- num-functions (function-def-falloff-param))]
                               [desired-depth (if (<= number-past 0)
                                                ;; reset the depth to 1 level deeper (to avoid infinite depth)
                                                (- 1 current-depth)
                                                ;; decrease the depth by half for every function past the falloff parameter
                                                ;; decrease = (current-depth / number-past) - 1
                                                (- (- (/ current-depth number-past) 1)))])
                          ;;[_ (printf "[new function] current depth: ~a  desired-depth: ~a\n" current-depth desired-depth)])
                          desired-depth))])


;; Returns a list of function references that the given function calls
(add-attribute
  wasmlike
  function-calls
  [#f (λ (n) (list))]
  [Func (λ (n) 
           (att-value 'xsmith_find-descendants n (λ (descendant) 
                                                    (equal? (node-type descendant)
                                                            'FunctionReference))))])

;; Do you or any of your children contain a reference to the originating function?
(define (recursive-reference? reference originating-function)
  (let* ([originating-function-name (ast-child 'name originating-function)]
         [reference-name (binding-name reference)] ;;Get the name
         [functions-node (ast-child 'functions (top-ancestor-node originating-function))]
         [function-node (att-value 'xsmith_find-a-descendant functions-node (λ (descendant)
                                                                               (and (equal? (node-type descendant)
                                                                                            'Func)
                                                                                    (equal? (ast-child 'name descendant)
                                                                                            reference-name))))]
         [function-calls (att-value 'function-calls function-node)]
         [function-call-names (map (λ (n) (binding-name (att-value 'xsmith_binding n)))
                                   function-calls)])
    (if (member originating-function-name function-call-names)
      #t ;; Recursive reference found
      ;;Otherwise, recursively traverse each of the other references
      (foldl (λ (a b) (or a b)) #f
             (map (λ (function-call)
                     (recursive-reference? (att-value 'xsmith_binding function-call) originating-function))
                  function-calls)))))


(add-property
 wasmlike
 reference-choice-info
 [FunctionReference (λ (n options lift-available?)
                       (let* ([current-function (list-ref (filter (λ (n)
                                                                     (equal? (node-type n) 'Func))
                                                                  (get-parents n)) 0)]
                              ;; First, avoid direct recursion
                              [options-without-self (filter (λ (n) (not (equal? (binding-name n)
                                                                                (ast-child 'name current-function))))
                                                            options)]
                              [options-without-recursion (filter (λ (n) (not (recursive-reference? n current-function)))
                                                                 options)])
                              ;;[_ (printf "current function name: ~a   options: ~a\n" (ast-child 'name current-function) options)])
                         (if (or (< (random) new-function-probability) (null? options-without-recursion))
                           'lift
                           (let* ([l (length options)]
                                  [choice (random 0 l)])
                             (list-ref options-without-recursion choice)))))])


; This will return a list of all addition and subtraction nodes
; It will not return any nodes that are subtypes, though!
;(att-value 'xsmith_find-descendants ast-node (λ (n) (member (ast-node-type n)
;                                                            '(AdditionExpression
;                                                              SubtractionExpression))))


;; All the base types of WebAssembly
(define i32 (base-type 'i32))
(define i64 (base-type 'i64))
(define f32 (base-type 'f32))
(define f64 (base-type 'f64))


;; Larger groups - use when ALL the contained types are valid
(define (fresh-number) (if (xsmith-feature-enabled? 'floating-point)
                         (fresh-type-variable i32 i64 f32 f64)
                         (fresh-type-variable i32 i64)))
(define (fresh-int) (fresh-type-variable i32 i64))
(define (fresh-float) (if (xsmith-feature-enabled? 'floating-point)
                        (fresh-type-variable f32 f64)
                        (begin
                          (printf "Should not get here. A floating point type was generated while the 'floating-point' feature was turned off")
                          #f)))

(define (no-child-types)
  (λ (n t)
     (hash)))

(define (binop-rhs) (λ (n t)
                       (hash 'l t
                             'r t)))
(define (unop-rhs) (λ (n t)
                      (hash 'expr t)))

(add-property
 wasmlike type-info
          [Program [i32
                    (λ (n t) 
                       (hash 'main (function-type (product-type (list)) i32)
                             'functions (λ (child-node) (function-type (product-type #f) (fresh-number)))
                             'globals (λ (child-node) (fresh-number))))]]
                             
          ;; function-type:  (function-type arg-type return-type)
          ;;   arg-type: single base type
          ;;   product-type: replacement for arg-type, contains a list. Ex: (product-type (list a b c))
          [Func [(function-type (product-type #f) (fresh-number))
                 (λ (n t)
                    (define arg-types (map (λ (param)
                                              (fresh-number))
                                           (ast-children (ast-child 'params n))))
                    (define f-type (function-type (product-type arg-types)
                                                  (fresh-type-variable)))
                    (unify! f-type t)
                    (define root-type (function-type-return-type! f-type))

                    (hash-set*
                      (for/hash ([p (ast-children (ast-child 'params n))]
                                 [p-type arg-types])
                        (values p p-type))
                      'root root-type
                      'locals (λ (child-node) (fresh-number))))]]
          [Param [(fresh-number) (no-child-types)]]

          [Call [(fresh-number)
                 (λ (n t)
                    ;; A call has the type of the return-type of its function, as well as
                    ;; the same argument types in the same order
                    ;; Set the type based on the function's return type
                    ;(eprintf "Starting 'Call type-info\n")
                    ;(unify! t (function-type-return-type!
                                ;(binding-type (att-value 'xsmith_binding (ast-child 'function n)))))
                    ;(eprintf "Successfully unified call's type with return type of function\n")
;
                    ;; The type-checking of arguments happens in the Arguments node
                    (define pt (product-type #f))
                    (hash 'argnode pt
                          ;; Function return and call type must match
                          'function (function-type pt t)))]]
          [IndirectCall [(fresh-number)
                         (λ (n t)
                            (define pt (product-type #f))
                            (hash 'argnode pt
                                  ;; Function return and call type
                                  'function (function-type pt t)))]]

          [FunctionReference [(function-type (product-type #f) (fresh-type-variable))
                              (no-child-types)]] 
                               ;;todo - check if I need to unify this with the reference
                              ;(λ (n t)
                                 ;(unify! t (binding-type (att-value 'xsmith_binding n))))]]

          [Arguments [(product-type #f)
                      (λ (n t)
                         ;; Get the argument list from the target function
                         (define pt (product-type #f))
                         (unify! pt t)
                         (when (not (product-type-inner-type-list! pt))
                           ;; Force exploration of function node to fill in args list.
                           (att-value 'xsmith_type (ast-child 'function
                                                              (ast-parent n))))
                         ;; Set the args for the call
                         (for/hash ([arg (ast-children (ast-child 'args n))]
                                    [arg-type (product-type-inner-type-list! pt)])
                           (values arg arg-type)))]]


          ;; Specify that the init and the type of the VariableDef must match
          [VariableDef [(fresh-number)
                        (λ (n t)
                           (hash 'init t))]]
          [VariableGet [(fresh-number)
                        (no-child-types)]]
          [VariableSet [(fresh-number)
                        (λ (n t)
                           (hash 'val t
                                 'expr t))]]
          [VariableTee [(fresh-number)
                        (λ (n t)
                           (hash 'val t))]]
          [DummyLiteral [(fresh-number)
                         (no-child-types)]]
          [LiteralIntThirtyTwo [i32
                                 (no-child-types)]]
          [LiteralIntSixtyFour [i64
                                 (no-child-types)]]
          [LiteralFloatThirtyTwo [f32
                                   (no-child-types)]]
          [LiteralFloatSixtyFour [f64
                                   (no-child-types)]]
          [Noop [(fresh-number) (λ (n t) (hash 'expr t))]] 
          
          [Binop [(fresh-number) (binop-rhs)]]
          ;; Restricted binops:
          [Remainder [(fresh-int) (binop-rhs)]]
          [And [(fresh-int) (binop-rhs)]]
          [Or [(fresh-int) (binop-rhs)]]
          [Xor [(fresh-int) (binop-rhs)]]
          [ShiftLeft [(fresh-int) (binop-rhs)]]
          [ShiftRight [(fresh-int) (binop-rhs)]]
          [RotateLeft [(fresh-int) (binop-rhs)]]
          [RotateRight [(fresh-int) (binop-rhs)]]
          [Min [(fresh-float) (binop-rhs)]]
          [Max [(fresh-float) (binop-rhs)]]
          [CopySign [(fresh-float) (binop-rhs)]]

          [Unop [(fresh-number) (unop-rhs)]]
          ;; Restricted Unops
          [CountLeadingZero [(fresh-int) (unop-rhs)]]
          [CountTrailingZero [(fresh-int) (unop-rhs)]]
          [NonZeroBits [(fresh-int) (unop-rhs)]]
          [AbsoluteValue [(fresh-float) (unop-rhs)]]
          [Negate [(fresh-float) (unop-rhs)]]
          [SquareRoot [(fresh-float) (unop-rhs)]]
          [Ceiling [(fresh-float) (unop-rhs)]]
          [Floor [(fresh-float) (unop-rhs)]]
          [Truncate [(fresh-float) (unop-rhs)]]
          [Nearest [(fresh-float) (unop-rhs)]]
          
          [Comparison [i32
                        (λ (n t) ;; The type of the children is unconstrained, they just have to be the same
                           (define child-type (fresh-number)) 
                           (hash 'l child-type
                                 'r child-type))]]
          [Testop [i32
                    (λ (n t) ;; The only testop in wasm 1.1 is integer only
                       (hash 'expr (fresh-int)))]]
          [Select [(fresh-number)
                   (λ (n t)
                      (hash 'l t
                            'r t
                            'selector i32))]]

          [IfElse [(fresh-number)
                    (λ (n t)
                     (hash 'cond i32
                           'then t
                           'else t))]]
          [Block [(fresh-number) (λ (n t) (hash 'expr t))]]
          ;;[Loop [(fresh-number) (λ (n t)  
          ;; can either use variable names and immediately put stuff into names or have to concretize types first before choosing type conversions
           ;                        (unify! (ast-child 'looptype n) t)
           ;                        (hash-set
           ;                          (for/hash ([param-expr (ast-children (ast-child 'paramexprs n))]
           ;                                     [param-type (ast-child 'paramtypes n)])
           ;                            (values param-expr param-type))
           ;                          'body t))]]

          [IdiomaticLoop [(fresh-number) (λ (n t)
                                            (hash 'initparam t
                                                   'body t))]]
          [Branch [(fresh-number) (λ (n t)
                                     (for/hash ([v (ast-children (ast-child 'vals n))]
                                                [t (product-type-inner-type-list! (ast-child 'targettypes n))])
                                       (values v t)))]]
          [BranchIf [(fresh-number) (λ (n t)
                                       (hash-set*
                                         (for/hash ([v (ast-children (ast-child 'vals n))]
                                                    [t (product-type-inner-type-list! (ast-child 'targettypes n))])
                                           (values v t))
                                         'cond i32
                                         'expr t))]]

          [StoreIntThirtyTwo [i32 (λ (n t) (hash 'value t
                                                 'expr t))]]
          [StoreFloatThirtyTwo [f32 (λ (n t) (hash 'value t
                                                   'expr t))]]
          [StoreIntSixtyFour [i64 (λ (n t) (hash 'value t
                                                 'expr t))]]
          [StoreFloatSixtyFour [f64 (λ (n t) (hash 'value t
                                                   'expr t))]]
          [StoreEight [(fresh-int)
                       (λ (n t) (hash 'value t
                                      'expr t))]]
          [StoreSixteen [(fresh-int)
                         (λ (n t) (hash 'value t
                                        'expr t))]]
          [StoreThirtyTwo [i64
                            (λ (n t) (hash 'value t
                                           'expr t))]]

          [LoadIntThirtyTwo [i32 (no-child-types)]]
          [LoadFloatThirtyTwo [f32 (no-child-types)]]
          [LoadIntSixtyFour [i64 (no-child-types)]]
          [LoadFloatSixtyFour [f64 (no-child-types)]]
          [LoadEightSigned [(fresh-int) (no-child-types)]]
          [LoadEightUnsigned [(fresh-int) (no-child-types)]]
          [LoadSixteenSigned [(fresh-int) (no-child-types)]]
          [LoadSixteenUnsigned [(fresh-int) (no-child-types)]]
          [LoadThirtyTwoSigned [i64 (no-child-types)]]
          [LoadThirtyTwoUnsigned [i64 (no-child-types)]]

          ;;Type conversions
          [TruncateFloat [(fresh-int) 
                     (λ (n t) 
                        (hash 'expr (fresh-float)))]]
          [ConvertInt [(fresh-float) 
                    (λ (n t) 
                       (hash 'expr (fresh-int)))]]
          [Wrap [i32 (λ (n t) 
                        (hash 'expr i64))]]
          [Extend [i64 (λ (n t)
                          (hash 'expr i32))]]
          [SignExtendSixtyFour [i64 (λ (n t)
                          (hash 'expr i64))]]
          [SignExtendThirtyTwo [i32 (λ (n t)
                          (hash 'expr i32))]]
          [Demote [f32 (λ (n t)
                          (hash 'expr f64))]]
          [Promote [f64 (λ (n t)
                          (hash 'expr f32))]]
          [ReinterpretIntThirtyTwo [f32 (λ (n t)
                                           (hash 'expr i32))]]
          [ReinterpretIntSixtyFour [f64 (λ (n t)
                                           (hash 'expr i64))]]
          [ReinterpretFloatThirtyTwo [i32 (λ (n t)
                                           (hash 'expr f32))]]
          [ReinterpretFloatSixtyFour [i64 (λ (n t)
                                           (hash 'expr f64))]]
)

;; Define structured control instruction property
(define-non-inheriting-rule-property
  structured-control-instruction
  attribute
  #:default (λ (n) #f)
  )

(add-property
 wasmlike
 structured-control-instruction
 [Func (λ (n) #t)]
 [IfElse (λ (n) #t)]
 [Block (λ (n) #t)]
 [IdiomaticLoop (λ (n) #t)])

;; gets an ancestry trace up the tree
(define (get-parents n)
  (if (parent-node n)
    (cons n (get-parents (parent-node n)))
    (list n)))

;; converts a list of nodes into a list of their names (for debugging)
(define (node-names l)
  (map (λ (n) (node-type n))
       l))

(define (valid-br-targets ns)
  (filter values
          (for/list ([child ns]
                     [parent (rest ns)])
            (if (and (att-value 'structured-control-instruction parent)
                     (att-value 'control-valid? parent child))
              parent
              #f))))

(add-attribute
  wasmlike
  control-valid?
  [#f (λ (parent child) #t)]
  [IfElse (λ (parent child)
             (not (equal? (ast-child 'cond parent) child)))]
  [IdiomaticLoop (λ (parent child)
           (not (equal? (ast-child 'initparam parent) child)))]
  )


(define (choose-br-target n)
  (let* ([parents (get-parents n)]
         [targets (valid-br-targets parents)]
         [index (random (length targets))])
    (cons index (list-ref targets index))))

;; Convenience function for getting the name of the type
;; This function is only used in the renderer, so it will concretize
;; and unify the type and return its name as a symbol
(define (get-base-type-name n)
  (let* ([nt (if (type? n)
                 n
                 (att-value 'xsmith_type n))]
         [concretized (concretize-type nt)])
    (unify! nt concretized)
    (base-type-name concretized)))

;; Combines an node's base type and a given string. Returns a symbol.
;; This is useful for constructing symbols like i32.add
;; Usage: (prefix-type <some node> ".add")
;; Return: 'i32.add
(define (prefix-type node instruction)
  (let ([type (get-base-type-name node)])
    (string->symbol (format "~a~a" type instruction))))

;; Returns either 'local or 'global when given a variable reference
(define (local-or-global-origin reference)
  (let* ([binding (att-value 'xsmith_binding reference)]
         [parent (parent-node (binding-ast-node binding))])
    (if (equal? (node-type parent) 'Func)
      'local
      'global)))

(add-property
 wasmlike
 render-node-info
 [Program (λ (n) 
             (when (debug-show-s-exp-param)
               (printf "S-expression representation of program:\n")
               (pretty-print
                 (att-value '_xsmith_to-s-expression n)
                 (current-output-port)
                 1)
               (printf "\n\n"))

             `(module
                    (import "env" "addToCrc" (func $addToCrc (param i32)))
                    (memory $mem 1)
                    (export "_memory" (memory $mem))
                    ;; Print generated globals
                    ,@(map (λ (global)
                             `(global ,(string->symbol (format "$~a" (ast-child 'name global))) 
                                      (mut ,(get-base-type-name global))
                                      ,($xsmith_render-node (ast-child 'init global))))
                           (ast-children (ast-child 'globals n)))


                    (table ,(length (att-value 'table-funcs n)) funcref)
                    (elem (i32.const 0) 
                    ,@(map (λ (indirect_call)
                           (string->symbol (string-append "$" (ast-child 'name (ast-child 'function indirect_call))))
                           )
                           (att-value 'table-funcs n)))
                    
                    
                    ;; Print the main function
                    ,($xsmith_render-node (ast-child 'main n))
                    ;; Print the other functions
                    ,@(map (λ (function)
                              ($xsmith_render-node function))
                           (ast-children (ast-child 'functions n)))
                    (func $crc_globals (export "_crc_globals") (local $storage i64)
                       ,@(flatten (map (λ (global) (append
                                                    '(global.get)
                                                    (list (string->symbol (format "$~a" (ast-child 'name global))))
                                                    ;;convert to i32
                                                    (let ([type (get-base-type-name global)])
                                                      (cond [(equal? 'i32 type)
                                                             '(call $addToCrc)]
                                                            [(equal? 'i64 type)
                                                             '(local.tee $storage
                                                               i32.wrap_i64
                                                               call $addToCrc
                                                               local.get $storage
                                                               i64.const 32
                                                               i64.shr_u
                                                               i32.wrap_i64
                                                               call $addToCrc)]
                                                            [(equal? 'f32 type)
                                                             '(i32.reinterpret_f32
                                                               call $addToCrc)]  
                                                            [(equal? 'f64 type)
                                                             '(i64.reinterpret_f64
                                                               local.tee $storage
                                                               i32.wrap_i64
                                                               call $addToCrc
                                                               local.get $storage
                                                               i64.const 32
                                                               i64.shr_u
                                                               i32.wrap_i64
                                                               call $addToCrc)]))))
                                       (reverse (ast-children (ast-child 'globals n))))))))]
 [Func (λ (n)
          `(,@(if (equal? (ast-child 'name n) "main")
                ;; The main function is printed a little differently, with a named export
                `(func $main (export "_main"))
                `(func ,(string->symbol (format "$~a" (ast-child 'name n)))))
             ;; Print parameters
             ,@(map (λ (p)
                       `(param ,(string->symbol (format "$~a" (ast-child 'name p))) ,(get-base-type-name p)))
                    (ast-children (ast-child 'params n)))
             ;; Print function type
             ,(if (equal? (ast-child 'name n) "main")
                '(result i32)
                `(result ,(get-base-type-name
                            (function-type-return-type!
                              (concretize-type (att-value 'xsmith_type n))))))
             ;; Print 'duplication' locals. These are needed for dynamic checks to avoid computing a value twice
             (local $i32_storage i32)
             (local $i64_storage i64)
             (local $f32_storage f32)
             (local $f64_storage f64)

             ;; Print locals. Ignore init value in VariableDef
             ,@(map (λ (l)
                       `(local ,(string->symbol (format "$~a" (ast-child 'name l))) ,(get-base-type-name l)))
                    (ast-children (ast-child 'locals n)))
              ;; Print idiomatic loop storage locals
              ,@(map (λ (l)
                       `(local ,(string->symbol (format "$~a_storage" (ast-child 'name l))) ,(get-base-type-name (ast-child 'initparam l))))
                    (att-value 'xsmith_find-descendants n (λ (d) (equal? (node-type d) 'IdiomaticLoop))))
              ;; Print idiomatic loop counter locals
              ,@(map (λ (l)
                       `(local ,(string->symbol (format "$~a_counter" (ast-child 'name l))) i32))
                    (att-value 'xsmith_find-descendants n (λ (d) (equal? (node-type d) 'IdiomaticLoop))))
             ;; Print root expression
             ,@($xsmith_render-node (ast-child 'root n))))]
 [Call (λ (n) 
          (append
            ;; Arguments first. The list is already reversed
            (append*  ;;unwrap the arguments list by 1
              (map (λ (arg)
                      ($xsmith_render-node arg))
                   (ast-children (ast-child 'args (ast-child 'argnode n)))))
            ;; Make the call
            (list 'call (string->symbol (format "$~a" (ast-child 'name (ast-child 'function n)))))))]
 [IndirectCall (λ (n) 
          (append
            (append*  ;;unwrap the arguments list by 1
              (map (λ (arg)
                      ($xsmith_render-node arg))
                   (ast-children (ast-child 'args (ast-child 'argnode n)))))
            ;; Make the call
            (list 'i32.const (index-of (att-value 'table-funcs (top-ancestor-node n)) n) 'call_indirect)
            (map (λ (p)
              `(param ,(get-base-type-name p)))
                (ast-children (ast-child 'params (binding-ast-node (att-value 'xsmith_binding (ast-child 'function n))))))
            (list `(result ,(get-base-type-name
                            (function-type-return-type!
                              (concretize-type (att-value 'xsmith_type (binding-ast-node (att-value 'xsmith_binding (ast-child 'function n)))))))))))]
 ;; VariableDefs are printed in the 'Program and 'Function nodes
 [VariableGet (λ (n) 
                 (let ([instruction (case (local-or-global-origin n)
                                      [(global) 'global.get]
                                      [(local) 'local.get])])
                   (append
                     `(,(values instruction) ,(string->symbol (format "$~a" (ast-child 'name n)))))))]
 [VariableSet (λ (n)
                 (let ([instruction (case (local-or-global-origin n)
                                      [(global) 'global.set]
                                      [(local) 'local.set])])
                   (append
                     ($xsmith_render-node (ast-child 'val n))
                     `(,(values instruction) ,(string->symbol (format "$~a" (ast-child 'name n))))
                     ($xsmith_render-node (ast-child 'expr n)))))]
 [VariableTee (λ (n) ;; Differs based on local or global: local has a tee instruction, global does not
                 (if (equal? 'global (local-or-global-origin n))
                   (append 
                     ($xsmith_render-node (ast-child 'val n))
                     `(global.set ,(string->symbol (format "$~a" (ast-child 'name n))))
                     `(global.get ,(string->symbol (format "$~a" (ast-child 'name n)))))
                   (append
                     ($xsmith_render-node (ast-child 'val n))
                     `(local.tee ,(string->symbol (format "$~a" (ast-child 'name n)))))))]
 [DummyLiteral (λ (n) (list ))] ;; Empty list in order to not print anything
 [LiteralIntThirtyTwo (λ (n) (list 'i32.const (ast-child 'v n)))]
 [LiteralIntSixtyFour (λ (n) (list 'i64.const (ast-child 'v n)))]
 [LiteralFloatThirtyTwo (λ (n) (list 'f32.const (ast-child 'v n)))]
 [LiteralFloatSixtyFour (λ (n) (list 'f64.const (ast-child 'v n)))]
 [Noop (λ (n) (append
                 '(nop)
                 ($xsmith_render-node (ast-child 'expr n))))]
 [Binop (λ (n) (cond [(equal? (node-type n) 'Division)
                              ;; For division, make sure that the divisor is not 0. We want to avoid crashes
                              ;; and 0/0 (NaN).
                              ;; For floats, we take divisor := abs(divisor) + 1
                              ;; For ints, we use select to choose either the original divisor or 1
                              (let ([type (get-base-type-name n)])
                                (if (or (equal? type 'i32) (equal? type 'i64))
                                  (append ;; int
                                    ;; don't allow division by -1 (MININT/-1 => NaN)
                                    ;;TODO - Add generation of minint and maxint
                                    ;; Left operand
                                    ($xsmith_render-node (ast-child 'l n))
                                    
                                    (list (prefix-type n ".const") '1)    ;; chosen if divisor == 0
                                  
                                    (list (prefix-type n ".const")) '(-2)   ;; chosen if divisor == -1
                                    ($xsmith_render-node (ast-child 'r n))  ;; Store the right operand
                                    `(local.tee ,(string->symbol (format "$~a_storage" type))) ;; Chosen if divisor != -1
                                    `(local.get ,(string->symbol (format "$~a_storage" type))) ;; doing checking operations on this one
                                    (list (prefix-type n ".const")) '(1) ;; check the divisor, add 1 to bring it to 0 if its -1
                                    (list (prefix-type n ".add"))
                                    (list (prefix-type n ".eqz")) ;; convert the divisor to i32 for select (which inverts the condition)
                                    '(select)

                                    ;; At this point, the divisor is on the stack. Store it and repeat
                                    ;; for the next check

                                    ;; don't allow division by 0
                                    ;;($xsmith_render-node (ast-child 'l n)) ;; Already on the stack from the previous check
                                    `(local.tee ,(string->symbol (format "$~a_storage" type))) ;; Chosen if divisor != 0
                                    `(local.get ,(string->symbol (format "$~a_storage" type))) ;; Doing checking operation on this one
                                    (list (prefix-type n ".eqz")) ;; convert the divisor to i32 for select (which inverts the condition)
                                    '(select)
                                    (list (prefix-type n (att-value 'math-op n))))
                                  (append ;; float
                                    ($xsmith_render-node (ast-child 'l n))
                                    ($xsmith_render-node (ast-child 'r n))
                                    (list (prefix-type n ".abs"))
                                    (list (prefix-type n ".const")) '(1)
                                    (list (prefix-type n ".add"))
                                    (list (prefix-type n (att-value 'math-op n))))))]
                     [(equal? (node-type n) 'Remainder)
                              ;; remainder operator cannot have a divisor of 0
                              ;; See division for explanation
                              (append
                                ($xsmith_render-node (ast-child 'l n))

                                (list (prefix-type n ".const")) '(1)
                                ($xsmith_render-node (ast-child 'r n))
                                `(local.tee ,(string->symbol (format "$~a_storage" (get-base-type-name n))))
                                `(local.get ,(string->symbol (format "$~a_storage" (get-base-type-name n))))
                                (list (prefix-type n ".eqz"))
                                '(select)
                                (list (prefix-type n (att-value 'math-op n))))]
                     [else (append 
                             ($xsmith_render-node (ast-child 'l n))
                             ($xsmith_render-node (ast-child 'r n))
                             (list (prefix-type n (att-value 'math-op n))))]))]
 [Unop (λ (n) (if (equal? (node-type n) 'SquareRoot)
                  ;; Use absolute value on the operand to a square root (negative square root NaN bug)
                  ;; We're lucky that square root only happens with floats and that abs is also only available
                  ;; for floats. There is a proposal that's merged and in the works, but isn't in the release yet
                  (append 
                    ($xsmith_render-node (ast-child 'expr n))
                    (list (prefix-type n ".abs"))
                    (list (prefix-type n (att-value 'math-op n))))
                  ;; Everything else as normal
                  (append
                    ($xsmith_render-node (ast-child 'expr n))
                    (list (prefix-type n (att-value 'math-op n))))))]
 [Comparison (λ (n)  (append
                       ($xsmith_render-node (ast-child 'l n))
                       ($xsmith_render-node (ast-child 'r n))
                       ;;The type of the comparison is based on the childrens' type
                       (list (prefix-type (ast-child 'l n) (att-value 'math-op n)))))]
 [Testop (λ (n) (append
                   ($xsmith_render-node (ast-child 'expr n))
                   (list (prefix-type (ast-child 'expr n) (att-value 'math-op n)))))]
 [Select (λ (n) (append
                   ($xsmith_render-node (ast-child 'l n))
                   ($xsmith_render-node (ast-child 'r n))
                   ($xsmith_render-node (ast-child 'selector n))
                   '(select)
                   ))]
 [IfElse (λ (n)           
           (append
                ($xsmith_render-node (ast-child 'cond n))
                `(if (result ,(get-base-type-name n)))
                ($xsmith_render-node (ast-child 'then n))
                '(else)
                ($xsmith_render-node (ast-child 'else n))
                '(end)))]
 [Block (λ (n)
           (append
             `(block (result ,(get-base-type-name n)))
             ($xsmith_render-node (ast-child 'expr n))
             '(end)))]
 [IdiomaticLoop (λ (n)
          (define loop-type (get-base-type-name (ast-child 'initparam n)))
          (define storage-name (string->symbol (string-append "$" (ast-child 'name n) "_storage")))
          (define counter-name (string->symbol (string-append "$" (ast-child 'name n) "_counter")))          
          (if (xsmith-feature-enabled? 'loop-parameters)
          (append
            ;; Counter setup
            `(i32.const ,(ast-child 'iterations n))
            `(local.set ,counter-name)

            ;; Param expression
            ($xsmith_render-node (ast-child 'initparam n))

            ;; Start of the loop
            `(loop (param ,loop-type) (result ,loop-type))
            ($xsmith_render-node (ast-child 'body n))

            ;; Decrement the counter 
            `(local.get ,counter-name)
            '(i32.const -1)
            '(i32.add)
            `(local.tee ,counter-name)
            ;; Conditional branch to the top of the loop
            '(br_if 0)
            '(end))
          (append
            ;; Counter setup
            `(i32.const ,(ast-child 'iterations n))
            `(local.set ,counter-name)

            ;; Param expression
            ($xsmith_render-node (ast-child 'initparam n))
            
            `(local.set ,storage-name)
            ;; Start of the loop
            `(loop)
            `(local.get ,storage-name)
            ($xsmith_render-node (ast-child 'body n))
            `(local.set ,storage-name)
            ;; Decrement the counter 
            `(local.get ,counter-name)
            '(i32.const -1)
            '(i32.add)
            `(local.tee ,counter-name)
            ;; Conditional branch to the top of the loop
            '(br_if 0)
            '(end)
            `(local.get ,storage-name))))]

 ;;[Loop (λ (n)
 ;         (append
 ;           ;; Param expressions
 ;           (append*
 ;             (map (λ (pe)
 ;                      ($xsmith_render-node pe))
 ;                  (ast-children (ast-child 'paramexprs n))))
 ;           `(loop              
 ;              ;; Print parameter typeblock
 ;              ,@(map (λ (p)
 ;                        `(param ,(get-base-type-name p)))
 ;                     (ast-child 'paramtypes n))
 ;              ;; Result type
 ;              (result ,(get-base-type-name n)))
 ;
 ;           ($xsmith_render-node (ast-child 'body n))
 ;           '(end)))]
 [Branch (λ (n)
            (append
              (append*
                (map (λ (v)
                        ($xsmith_render-node v))
                     (ast-children (ast-child 'vals n))))
              `(br ,(ast-child 'targetindex n))))]
 [BranchIf (λ (n)
              (append
                (append*
                  (map (λ (v)
                          ($xsmith_render-node v))
                       (ast-children (ast-child 'vals n))))
                ($xsmith_render-node (ast-child 'cond n))
                `(br_if ,(ast-child 'targetindex n))
                ;; If the branch isn't taken, the vals haven't been consumed. Drop them
                ;; to keep the shape of the stack.
                (map (λ (v)
                        'drop)
                     (ast-children (ast-child 'vals n)))
                ($xsmith_render-node (ast-child 'expr n))))]
 [MemStore (λ (n)
              (append
                `(i32.const ,(ast-child 'address n))
                ($xsmith_render-node (ast-child 'value n))
                `(,(prefix-type n (att-value 'mem-store-op n))
                   ,(string->symbol (format "offset=~a" (ast-child 'offset n))) 
                   ,(string->symbol (format "align=~a" (expt 2 (ast-child 'alignment n)))))
                ($xsmith_render-node (ast-child 'expr n))))]
 [MemLoad (λ (n)
              (let ([type (get-base-type-name n)])
                ;; if safe-memory-loads is turned on, extra instructions will be added after loading floats
                ;; to make sure they are not NaN or +/-infinity
                (if (and (equal? type 'f32) (xsmith-feature-enabled? 'safe-memory-loads)) 
                  (append ;; safe load f32
                    (list (prefix-type n ".const") (choose-random-float)) ;; used if loaded float is NaN or +/-inf
                    `(i32.const ,(ast-child 'address n))
                    `(,(prefix-type n (att-value 'mem-load-op n))
                        ,(string->symbol (format "offset=~a" (ast-child 'offset n))) 
                        ,(string->symbol (format "align=~a" (expt 2 (ast-child 'alignment n)))))
                    `(local.tee ,(string->symbol (format "$~a_storage" type)))
                    `(local.get ,(string->symbol (format "$~a_storage" type))) ;; duplicate float for safety check
                    `(i32.reinterpret_f32)
                    `(i32.const 0x7F800000)
                    `(i32.and)
                    `(i32.popcnt)
                    `(i32.const 8)
                    `(i32.eq) ;; true iff all bits in exponent of float are 1
                    `(select)
                    `(local.tee ,(string->symbol (format "$~a_storage" type)))) ;; overwrite local in case it was NaN
                  (if (and (equal? type 'f64) (xsmith-feature-enabled? 'safe-memory-loads))
                    (append ;; safe load f64
                    (list (prefix-type n ".const") (choose-random-float)) ;; used if loaded float is NaN or +/-inf
                    `(i32.const ,(ast-child 'address n))
                    `(,(prefix-type n (att-value 'mem-load-op n))
                        ,(string->symbol (format "offset=~a" (ast-child 'offset n))) 
                        ,(string->symbol (format "align=~a" (expt 2 (ast-child 'alignment n)))))
                    `(local.tee ,(string->symbol (format "$~a_storage" type)))
                    `(local.get ,(string->symbol (format "$~a_storage" type))) ;; duplicate float for safety check
                    `(i64.reinterpret_f64)
                    `(i64.const 0x7FF0000000000000)
                    `(i64.and)
                    `(i64.popcnt)
                    `(i64.const 11)
                    `(i64.eq)  ;; true iff all bits in exponent of float are 1
                    `(select)
                    `(local.tee ,(string->symbol (format "$~a_storage" type)))) ;; overwrite local in case it was NaN
                    (append ;; unsafe load i32, i64, f32, f64
                      `(i32.const ,(ast-child 'address n))
                      `(,(prefix-type n (att-value 'mem-load-op n))
                          ,(string->symbol (format "offset=~a" (ast-child 'offset n))) 
                          ,(string->symbol (format "align=~a" (expt 2 (ast-child 'alignment n))))))))))]
 [TruncateFloat (λ (n)
                   (append
                     ($xsmith_render-node (ast-child 'expr n))
                     `(,(let* ([prefix-type (get-base-type-name n)] ;;todo: problem here with wrong types
                               [suffix-type (get-base-type-name (ast-child 'expr n))]
                               [instruction (format "~a.~a_~a" prefix-type 'trunc_sat suffix-type)])
                          (if (ast-child 'sign n)
                            (string->symbol (format "~a~a" instruction '_s))
                            (string->symbol (format "~a~a" instruction '_u)))))))]
 [ConvertInt (λ (n)
                (append
                  ($xsmith_render-node (ast-child 'expr n))
                  `(,(let* ([prefix-type (get-base-type-name n)]
                            [suffix-type (get-base-type-name (ast-child 'expr n))]
                            [instruction (format "~a.~a_~a" prefix-type 'convert suffix-type)])
                       (if (ast-child 'sign n)
                         (string->symbol (format "~a~a" instruction '_s))
                         (string->symbol (format "~a~a" instruction '_u)))))))]
 [Wrap (λ (n)
          (append
            ($xsmith_render-node (ast-child 'expr n))
            '(i32.wrap_i64)))]
 [Extend (λ (n) ;;todo: add support for different extension widths: Section 5.4.5, v1.1
            (append
              ($xsmith_render-node (ast-child 'expr n))
              `(,(if (ast-child 'sign n)
                   'i64.extend_i32_s
                   'i64.extend_i32_u))))]
 [SignExtendThirtyTwo (λ (n)
            (append
              ($xsmith_render-node (ast-child 'expr n))
              `(,(string->symbol (string-append "i32.extend" (ast-child 'width n) "_s")))))]
 [SignExtendSixtyFour (λ (n)
            (append
              ($xsmith_render-node (ast-child 'expr n))
              `(,(string->symbol (string-append "i64.extend" (ast-child 'width n) "_s")))))]
 [Demote (λ (n)
            (append
              ($xsmith_render-node (ast-child 'expr n))
              '(f32.demote_f64)))]
 [Promote (λ (n)
             (append
               ($xsmith_render-node (ast-child 'expr n))
               '(f64.promote_f32)))]
 [ReinterpretIntThirtyTwo (λ (n)
                             (append
                               ($xsmith_render-node (ast-child 'expr n))
                               '(i32.const 2) ;;Shift right by 2 to mask off the sign bit and the first bit of the exponent
                               '(i32.shr_u)   ;;since a small negative number looks like a NaN
                               '(f32.reinterpret_i32)))]
 [ReinterpretIntSixtyFour (λ (n)
                             (append
                               ($xsmith_render-node (ast-child 'expr n))
                               '(i64.const 2)
                               '(i64.shr_u)
                               '(f64.reinterpret_i64)))]
 [ReinterpretFloatThirtyTwo (λ (n)
                               (append
                                 ($xsmith_render-node (ast-child 'expr n))
                                 '(i32.reinterpret_f32)))]
 [ReinterpretFloatSixtyFour (λ (n)
                               (append
                                 ($xsmith_render-node (ast-child 'expr n))
                                 '(i64.reinterpret_f64)))]
)

;; Convenience function to get the type of the node
;; Checks that the type is a valid leaf node type, and converts
;; the result into a symbol for ease of use in the renderer

(add-property
 wasmlike
 render-hole-info
 [#f (λ (n)
        (append 
          `(,(string->symbol "<HOLE>"))))])

(add-attribute
  wasmlike math-op
  [Addition (λ (n) '.add)]
  [Subtraction (λ (n) '.sub)]
  [Multiplication (λ (n) '.mul)]
  [Division (λ (n) (add-signed-suffix n '.div))]
  [Remainder (λ (n) (add-signed-suffix n '.rem))]
  [And (λ (n) '.and)]
  [Or (λ (n) '.or)]
  [Xor (λ (n) '.xor)]
  [ShiftLeft (λ (n) '.shl)]
  [ShiftRight (λ (n) (add-signed-suffix n '.shr))]
  [RotateLeft (λ (n) '.rotl)]
  [RotateRight (λ (n) '.rotr)]
  [Min (λ (n) '.min)]
  [Max (λ (n) '.max)]
  [CopySign (λ (n) '.copysign)]
  [Equal (λ (n) '.eq)]
  [NotEqual (λ (n) '.ne)]
  [LessThan (λ (n) (add-comparison-signed-suffix n (ast-child 'l n) '.lt))]
  [GreaterThan (λ (n) (add-comparison-signed-suffix n (ast-child 'l n) '.gt))]
  [LessThanOrEqual (λ (n) (add-comparison-signed-suffix n (ast-child 'l n) '.le))]
  [GreaterThanOrEqual (λ (n) (add-comparison-signed-suffix n (ast-child 'l n) '.ge))]
  [CountLeadingZero (λ (n) '.clz)]
  [CountTrailingZero (λ (n) '.ctz)]
  [NonZeroBits (λ (n) '.popcnt)]
  [AbsoluteValue (λ (n) '.abs)]
  [Negate (λ (n) '.neg)]
  [SquareRoot (λ (n) '.sqrt)]
  [Ceiling (λ (n) '.ceil)]
  [Floor (λ (n) '.floor)]
  [Truncate (λ (n) '.trunc)]
  [Nearest (λ (n) '.nearest)]
 
  [EqualZero (λ (n) '.eqz)])

(add-attribute
  wasmlike mem-load-op
  [LoadIntThirtyTwo (λ (n) '.load)]
  [LoadFloatThirtyTwo (λ (n) '.load)]
  [LoadIntSixtyFour (λ (n) '.load)]
  [LoadFloatSixtyFour (λ (n) '.load)]
  [LoadEightSigned (λ (n) '.load8_s)]
  [LoadEightUnsigned (λ (n) '.load8_u)]
  [LoadSixteenSigned (λ (n) '.load16_s)]
  [LoadSixteenUnsigned (λ (n) '.load16_u)]
  [LoadThirtyTwoSigned (λ (n) '.load32_s)]
  [LoadThirtyTwoUnsigned (λ (n) '.load32_u)])

(add-attribute
  wasmlike mem-store-op
  [StoreIntThirtyTwo (λ (n) '.store)]
  [StoreFloatThirtyTwo (λ (n) '.store)]
  [StoreIntSixtyFour (λ (n) '.store)]
  [StoreFloatSixtyFour (λ (n) '.store)]
  [StoreEight (λ (n) '.store8)]
  [StoreSixteen (λ (n) '.store16)]
  [StoreThirtyTwo (λ (n) '.store32)])

;; Adds a sign suffix if needed. It can differentiate between float and int for 
;; nodes like division, which only need a sign suffix on the int base-type.
(define (add-signed-suffix node instruction)
  (if (not (ast-has-child? 'sign node))
    (error 'add-signed-suffix (format "Node did not have a 'sign child to query: ~a\n"
                                      node))
    (let ([node-type (get-base-type-name node)])
      (cond [(or (equal? node-type 'i32) (equal? node-type 'i64))
             (if (ast-child 'sign node)
               (string->symbol (format "~a~a" instruction '_s))
               (string->symbol (format "~a~a" instruction '_u)))]
            [(or (equal? node-type 'f32) (equal? node-type 'f64))
             instruction] ;; floats don't need any suffixes
            [else
              (error 'add-signed-suffix (format "Node type not a base type when adding a signed suffix: ~a\n"
                                                node))]))))

;; Adds a sign suffix if needed for comparison operators. Both the comparison node and one of
;; of the children are needed: the parent for the sign, and the child for the type
(define (add-comparison-signed-suffix node child-node instruction)
  (if (not (ast-has-child? 'sign node))
    (error 'add-comparison-signed-suffix (format "Comparison node did not have a 'sign child to query: ~a\n"
                                                 node))
    (let ([child-node-type (get-base-type-name child-node)])
      (cond [(or (equal? child-node-type 'i32) (equal? child-node-type 'i64))
             (if (ast-child 'sign node)
               (string->symbol (format "~a~a" instruction '_s))
               (string->symbol (format "~a~a" instruction '_u)))]
            [(or (equal? child-node-type 'f32) (equal? child-node-type 'f64))
             instruction] ;; floats don't need any suffixes
            [else
              (error 'add-signed-suffix (format "Child node type not a base type when adding a signed suffix: ~a\n"
                                                child-node))]))))


(define wasmlike-version-string (string-append 
                                  (number->string wasmlike-version)
                                  " ("
                                  (string-trim 
                                      (with-output-to-string
                                        (λ () 
                                           (current-directory (path-only (path->complete-path (find-system-path 'run-file))))
                                           (system "git rev-parse --short HEAD"))))
                                    ")"))

(define-xsmith-interface-functions
  [wasmlike]
  #:fuzzer-name wasmlike
  #:fuzzer-version wasmlike-version-string
  #:program-node Program
  #:type-thunks (list (λ () i32)
                      (λ () i64)
                      (λ () (and (xsmith-feature-enabled? 'floating-point) f32))
                      (λ () (and (xsmith-feature-enabled? 'floating-point) f64)))
  #:comment-wrap (λ (lines)
                    (string-join
                      (map (λ (x) (format ";; ~a" x))
                           lines)
                      "\n"))
  #:format-render (λ (s-exp)
                     (substring
                       (pretty-format s-exp)
                       1))
  #:extra-parameters ([function-definition-falloff
                       "The number of function definitions that can be generated before falloff is
    applied, limiting future function definitions. (Defaults to 10)"
                       function-def-falloff-param 
                       string->number]
                      [debug-show-s-exp-tree
                       "Prints an s-expression tree of the program for debugging purposes"
                       debug-show-s-exp-param
                       string->bool])
  #:features ([floating-point #t]
              [sign-extension #t]
              [indirect-calls #t]
              [non-trapping-float-to-int #t]
              [loop-parameters #f]
              [safe-memory-loads #t]))



(module+ main (wasmlike-command-line))


;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;; End of file.

