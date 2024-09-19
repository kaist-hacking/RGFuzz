//! Implementation of a standard Riscv64 ABI.

use crate::ir;
use crate::ir::types::*;

use crate::ir::ExternalName;
use crate::ir::MemFlags;
use crate::isa;

use crate::isa::riscv64::{inst::EmitState, inst::*};
use crate::isa::CallConv;
use crate::machinst::*;

use crate::ir::types::I8;
use crate::ir::LibCall;
use crate::ir::Signature;
use crate::isa::riscv64::settings::Flags as RiscvFlags;
use crate::isa::unwind::UnwindInst;
use crate::settings;
use crate::CodegenError;
use crate::CodegenResult;
use alloc::boxed::Box;
use alloc::vec::Vec;
use regalloc2::{MachineEnv, PReg, PRegSet};
use regs::x_reg;

use smallvec::{smallvec, SmallVec};
use std::sync::OnceLock;

/// Support for the Riscv64 ABI from the callee side (within a function body).
pub(crate) type Riscv64Callee = Callee<Riscv64MachineDeps>;

/// Support for the Riscv64 ABI from the caller side (at a callsite).
pub(crate) type Riscv64ABICallSite = CallSite<Riscv64MachineDeps>;

/// This is the limit for the size of argument and return-value areas on the
/// stack. We place a reasonable limit here to avoid integer overflow issues
/// with 32-bit arithmetic: for now, 128 MB.
static STACK_ARG_RET_SIZE_LIMIT: u32 = 128 * 1024 * 1024;

/// Riscv64-specific ABI behavior. This struct just serves as an implementation
/// point for the trait; it is never actually instantiated.
pub struct Riscv64MachineDeps;

impl IsaFlags for RiscvFlags {}

impl RiscvFlags {
    pub(crate) fn min_vec_reg_size(&self) -> u64 {
        let entries = [
            (self.has_zvl65536b(), 65536),
            (self.has_zvl32768b(), 32768),
            (self.has_zvl16384b(), 16384),
            (self.has_zvl8192b(), 8192),
            (self.has_zvl4096b(), 4096),
            (self.has_zvl2048b(), 2048),
            (self.has_zvl1024b(), 1024),
            (self.has_zvl512b(), 512),
            (self.has_zvl256b(), 256),
            // In order to claim the Application Profile V extension, a minimum
            // register size of 128 is required. i.e. V implies Zvl128b.
            (self.has_v(), 128),
            (self.has_zvl128b(), 128),
            (self.has_zvl64b(), 64),
            (self.has_zvl32b(), 32),
        ];

        for (has_flag, size) in entries.into_iter() {
            if !has_flag {
                continue;
            }

            // Due to a limitation in regalloc2, we can't support types
            // larger than 1024 bytes. So limit that here.
            return std::cmp::min(size, 1024);
        }

        return 0;
    }
}

impl ABIMachineSpec for Riscv64MachineDeps {
    type I = Inst;
    type F = RiscvFlags;

    fn word_bits() -> u32 {
        64
    }

    /// Return required stack alignment in bytes.
    fn stack_align(_call_conv: isa::CallConv) -> u32 {
        16
    }

    fn compute_arg_locs<'a, I>(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        params: I,
        args_or_rets: ArgsOrRets,
        add_ret_area_ptr: bool,
        mut args: ArgsAccumulator<'_>,
    ) -> CodegenResult<(u32, Option<usize>)>
    where
        I: IntoIterator<Item = &'a ir::AbiParam>,
    {
        // All registers that can be used as parameters or rets.
        // both start and end are included.
        let (x_start, x_end, f_start, f_end) = match (call_conv, args_or_rets) {
            (isa::CallConv::Tail, _) => (9, 29, 0, 31),
            (_, ArgsOrRets::Args) => (10, 17, 10, 17),
            (_, ArgsOrRets::Rets) => (10, 11, 10, 11),
        };
        let mut next_x_reg = x_start;
        let mut next_f_reg = f_start;
        // Stack space.
        let mut next_stack: u32 = 0;

        for param in params {
            if let ir::ArgumentPurpose::StructArgument(size) = param.purpose {
                let offset = next_stack;
                assert!(size % 8 == 0, "StructArgument size is not properly aligned");
                next_stack += size;
                args.push(ABIArg::StructArg {
                    pointer: None,
                    offset: offset as i64,
                    size: size as u64,
                    purpose: param.purpose,
                });
                continue;
            }

            // Find regclass(es) of the register(s) used to store a value of this type.
            let (rcs, reg_tys) = Inst::rc_for_type(param.value_type)?;
            let mut slots = ABIArgSlotVec::new();
            for (rc, reg_ty) in rcs.iter().zip(reg_tys.iter()) {
                let next_reg = if (next_x_reg <= x_end) && *rc == RegClass::Int {
                    let x = Some(x_reg(next_x_reg));
                    next_x_reg += 1;
                    x
                } else if (next_f_reg <= f_end) && *rc == RegClass::Float {
                    let x = Some(f_reg(next_f_reg));
                    next_f_reg += 1;
                    x
                } else {
                    None
                };
                if let Some(reg) = next_reg {
                    slots.push(ABIArgSlot::Reg {
                        reg: reg.to_real_reg().unwrap(),
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                } else {
                    // Compute size and 16-byte stack alignment happens
                    // separately after all args.
                    let size = reg_ty.bits() / 8;
                    let size = std::cmp::max(size, 8);
                    // Align.
                    debug_assert!(size.is_power_of_two());
                    next_stack = align_to(next_stack, size);
                    slots.push(ABIArgSlot::Stack {
                        offset: next_stack as i64,
                        ty: *reg_ty,
                        extension: param.extension,
                    });
                    next_stack += size;
                }
            }
            args.push(ABIArg::Slots {
                slots,
                purpose: param.purpose,
            });
        }
        let pos: Option<usize> = if add_ret_area_ptr {
            assert!(ArgsOrRets::Args == args_or_rets);
            if next_x_reg <= x_end {
                let arg = ABIArg::reg(
                    x_reg(next_x_reg).to_real_reg().unwrap(),
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
            } else {
                let arg = ABIArg::stack(
                    next_stack as i64,
                    I64,
                    ir::ArgumentExtension::None,
                    ir::ArgumentPurpose::Normal,
                );
                args.push(arg);
                next_stack += 8;
            }
            Some(args.args().len() - 1)
        } else {
            None
        };

        next_stack = align_to(next_stack, Self::stack_align(call_conv));

        // To avoid overflow issues, limit the arg/return size to something
        // reasonable -- here, 128 MB.
        if next_stack > STACK_ARG_RET_SIZE_LIMIT {
            return Err(CodegenError::ImplLimitExceeded);
        }

        Ok((next_stack, pos))
    }

    fn fp_to_arg_offset(_call_conv: isa::CallConv, _flags: &settings::Flags) -> i64 {
        // lr fp.
        16
    }

    fn gen_load_stack(mem: StackAMode, into_reg: Writable<Reg>, ty: Type) -> Inst {
        Inst::gen_load(into_reg, mem.into(), ty, MemFlags::trusted())
    }

    fn gen_store_stack(mem: StackAMode, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_store(mem.into(), from_reg, ty, MemFlags::trusted())
    }

    fn gen_move(to_reg: Writable<Reg>, from_reg: Reg, ty: Type) -> Inst {
        Inst::gen_move(to_reg, from_reg, ty)
    }

    fn gen_extend(
        to_reg: Writable<Reg>,
        from_reg: Reg,
        signed: bool,
        from_bits: u8,
        to_bits: u8,
    ) -> Inst {
        assert!(from_bits < to_bits);
        Inst::Extend {
            rd: to_reg,
            rn: from_reg,
            signed,
            from_bits,
            to_bits,
        }
    }

    fn get_ext_mode(
        _call_conv: isa::CallConv,
        specified: ir::ArgumentExtension,
    ) -> ir::ArgumentExtension {
        specified
    }

    fn gen_args(args: Vec<ArgPair>) -> Inst {
        Inst::Args { args }
    }

    fn gen_rets(rets: Vec<RetPair>) -> Inst {
        Inst::Rets { rets }
    }

    fn get_stacklimit_reg(_call_conv: isa::CallConv) -> Reg {
        spilltmp_reg()
    }

    fn gen_add_imm(
        _call_conv: isa::CallConv,
        into_reg: Writable<Reg>,
        from_reg: Reg,
        imm: u32,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallInstVec::new();
        if let Some(imm12) = Imm12::maybe_from_u64(imm as u64) {
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Addi,
                rd: into_reg,
                rs: from_reg,
                imm12,
            });
        } else {
            insts.extend(Inst::load_constant_u32(
                writable_spilltmp_reg2(),
                imm as u64,
            ));
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Add,
                rd: into_reg,
                rs1: spilltmp_reg2(),
                rs2: from_reg,
            });
        }
        insts
    }

    fn gen_stack_lower_bound_trap(limit_reg: Reg) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();
        insts.push(Inst::TrapIf {
            cc: IntCC::UnsignedLessThan,
            rs1: stack_reg(),
            rs2: limit_reg,
            trap_code: ir::TrapCode::StackOverflow,
        });
        insts
    }

    fn gen_get_stack_addr(mem: StackAMode, into_reg: Writable<Reg>, _ty: Type) -> Inst {
        Inst::LoadAddr {
            rd: into_reg,
            mem: mem.into(),
        }
    }

    fn gen_load_base_offset(into_reg: Writable<Reg>, base: Reg, offset: i32, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_load(into_reg, mem, ty, MemFlags::trusted())
    }

    fn gen_store_base_offset(base: Reg, offset: i32, from_reg: Reg, ty: Type) -> Inst {
        let mem = AMode::RegOffset(base, offset as i64, ty);
        Inst::gen_store(mem, from_reg, ty, MemFlags::trusted())
    }

    fn gen_sp_reg_adjust(amount: i32) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();

        if amount == 0 {
            return insts;
        }

        if let Some(imm) = Imm12::maybe_from_i64(amount as i64) {
            insts.push(Inst::AluRRImm12 {
                alu_op: AluOPRRI::Addi,
                rd: writable_stack_reg(),
                rs: stack_reg(),
                imm12: imm,
            })
        } else {
            let tmp = writable_spilltmp_reg();
            insts.extend(Inst::load_constant_u64(tmp, amount as i64 as u64));
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Add,
                rd: writable_stack_reg(),
                rs1: stack_reg(),
                rs2: tmp.to_reg(),
            });
        }

        insts
    }

    fn gen_nominal_sp_adj(offset: i32) -> Inst {
        Inst::VirtualSPOffsetAdj {
            amount: offset as i64,
        }
    }

    fn gen_prologue_frame_setup(
        _call_conv: isa::CallConv,
        flags: &settings::Flags,
        _isa_flags: &RiscvFlags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            // add  sp,sp,-16    ;; alloc stack space for fp.
            // sd   ra,8(sp)     ;; save ra.
            // sd   fp,0(sp)     ;; store old fp.
            // mv   fp,sp        ;; set fp to sp.
            insts.extend(Self::gen_sp_reg_adjust(-16));
            insts.push(Self::gen_store_stack(
                StackAMode::SPOffset(8, I64),
                link_reg(),
                I64,
            ));
            insts.push(Self::gen_store_stack(
                StackAMode::SPOffset(0, I64),
                fp_reg(),
                I64,
            ));

            if flags.unwind_info() {
                insts.push(Inst::Unwind {
                    inst: UnwindInst::PushFrameRegs {
                        offset_upward_to_caller_sp: frame_layout.setup_area_size,
                    },
                });
            }
            insts.push(Inst::Mov {
                rd: writable_fp_reg(),
                rm: stack_reg(),
                ty: I64,
            });
        }

        insts
    }
    /// reverse of gen_prologue_frame_setup.
    fn gen_epilogue_frame_restore(
        call_conv: isa::CallConv,
        _flags: &settings::Flags,
        _isa_flags: &RiscvFlags,
        frame_layout: &FrameLayout,
    ) -> SmallInstVec<Inst> {
        let mut insts = SmallVec::new();

        if frame_layout.setup_area_size > 0 {
            insts.push(Self::gen_load_stack(
                StackAMode::SPOffset(8, I64),
                writable_link_reg(),
                I64,
            ));
            insts.push(Self::gen_load_stack(
                StackAMode::SPOffset(0, I64),
                writable_fp_reg(),
                I64,
            ));
            insts.extend(Self::gen_sp_reg_adjust(16));
        }

        if call_conv == isa::CallConv::Tail && frame_layout.stack_args_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(
                frame_layout.stack_args_size.try_into().unwrap(),
            ));
        }
        insts.push(Inst::Ret {});

        insts
    }

    fn gen_probestack(insts: &mut SmallInstVec<Self::I>, frame_size: u32) {
        insts.extend(Inst::load_constant_u32(writable_a0(), frame_size as u64));
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Probestack),
                uses: smallvec![CallArgPair {
                    vreg: a0(),
                    preg: a0(),
                }],
                defs: smallvec![],
                clobbers: PRegSet::empty(),
                opcode: Opcode::Call,
                callee_callconv: CallConv::SystemV,
                caller_callconv: CallConv::SystemV,
                callee_pop_size: 0,
            }),
        });
    }

    fn gen_clobber_save(
        _call_conv: isa::CallConv,
        flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = SmallVec::new();
        // Adjust the stack pointer downward for clobbers and the function fixed
        // frame (spillslots and storage slots).
        let stack_size = frame_layout.fixed_frame_storage_size + frame_layout.clobber_size;
        if flags.unwind_info() && frame_layout.setup_area_size > 0 {
            // The *unwind* frame (but not the actual frame) starts at the
            // clobbers, just below the saved FP/LR pair.
            insts.push(Inst::Unwind {
                inst: UnwindInst::DefineNewFrame {
                    offset_downward_to_clobbers: frame_layout.clobber_size,
                    offset_upward_to_caller_sp: frame_layout.setup_area_size,
                },
            });
        }
        // Store each clobbered register in order at offsets from SP,
        // placing them above the fixed frame slots.
        if stack_size > 0 {
            // since we use fp, we didn't need use UnwindInst::StackAlloc.
            let mut cur_offset = 8;
            for reg in &frame_layout.clobbered_callee_saves {
                let r_reg = reg.to_reg();
                let ty = match r_reg.class() {
                    RegClass::Int => I64,
                    RegClass::Float => F64,
                    RegClass::Vector => unimplemented!("Vector Clobber Saves"),
                };
                if flags.unwind_info() {
                    insts.push(Inst::Unwind {
                        inst: UnwindInst::SaveReg {
                            clobber_offset: frame_layout.clobber_size - cur_offset,
                            reg: r_reg,
                        },
                    });
                }
                insts.push(Self::gen_store_stack(
                    StackAMode::SPOffset(-(cur_offset as i64), ty),
                    real_reg_to_reg(reg.to_reg()),
                    ty,
                ));
                cur_offset += 8
            }

            insts.extend(Self::gen_sp_reg_adjust(-(stack_size as i32)));
        }
        insts
    }

    fn gen_clobber_restore(
        _call_conv: isa::CallConv,
        _flags: &settings::Flags,
        frame_layout: &FrameLayout,
    ) -> SmallVec<[Inst; 16]> {
        let mut insts = SmallVec::new();
        let stack_size = frame_layout.fixed_frame_storage_size + frame_layout.clobber_size;
        if stack_size > 0 {
            insts.extend(Self::gen_sp_reg_adjust(stack_size as i32));
        }
        let mut cur_offset = 8;
        for reg in &frame_layout.clobbered_callee_saves {
            let rreg = reg.to_reg();
            let ty = match rreg.class() {
                RegClass::Int => I64,
                RegClass::Float => F64,
                RegClass::Vector => unimplemented!("Vector Clobber Restores"),
            };
            insts.push(Self::gen_load_stack(
                StackAMode::SPOffset(-cur_offset, ty),
                Writable::from_reg(real_reg_to_reg(reg.to_reg())),
                ty,
            ));
            cur_offset += 8
        }
        insts
    }

    fn gen_call(
        dest: &CallDest,
        uses: CallArgList,
        defs: CallRetList,
        clobbers: PRegSet,
        opcode: ir::Opcode,
        tmp: Writable<Reg>,
        callee_conv: isa::CallConv,
        caller_conv: isa::CallConv,
        callee_pop_size: u32,
    ) -> SmallVec<[Self::I; 2]> {
        let mut insts = SmallVec::new();
        match &dest {
            &CallDest::ExtName(ref name, RelocDistance::Near) => insts.push(Inst::Call {
                info: Box::new(CallInfo {
                    dest: name.clone(),
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                    callee_pop_size,
                }),
            }),
            &CallDest::ExtName(ref name, RelocDistance::Far) => {
                insts.push(Inst::LoadExtName {
                    rd: tmp,
                    name: Box::new(name.clone()),
                    offset: 0,
                });
                insts.push(Inst::CallInd {
                    info: Box::new(CallIndInfo {
                        rn: tmp.to_reg(),
                        uses,
                        defs,
                        clobbers,
                        opcode,
                        caller_callconv: caller_conv,
                        callee_callconv: callee_conv,
                        callee_pop_size,
                    }),
                });
            }
            &CallDest::Reg(reg) => insts.push(Inst::CallInd {
                info: Box::new(CallIndInfo {
                    rn: *reg,
                    uses,
                    defs,
                    clobbers,
                    opcode,
                    caller_callconv: caller_conv,
                    callee_callconv: callee_conv,
                    callee_pop_size,
                }),
            }),
        }
        insts
    }

    fn gen_memcpy<F: FnMut(Type) -> Writable<Reg>>(
        call_conv: isa::CallConv,
        dst: Reg,
        src: Reg,
        size: usize,
        mut alloc_tmp: F,
    ) -> SmallVec<[Self::I; 8]> {
        let mut insts = SmallVec::new();
        let arg0 = Writable::from_reg(x_reg(10));
        let arg1 = Writable::from_reg(x_reg(11));
        let arg2 = Writable::from_reg(x_reg(12));
        let tmp = alloc_tmp(Self::word_type());
        insts.extend(Inst::load_constant_u64(tmp, size as u64).into_iter());
        insts.push(Inst::Call {
            info: Box::new(CallInfo {
                dest: ExternalName::LibCall(LibCall::Memcpy),
                uses: smallvec![
                    CallArgPair {
                        vreg: dst,
                        preg: arg0.to_reg()
                    },
                    CallArgPair {
                        vreg: src,
                        preg: arg1.to_reg()
                    },
                    CallArgPair {
                        vreg: tmp.to_reg(),
                        preg: arg2.to_reg()
                    }
                ],
                defs: smallvec![],
                clobbers: Self::get_regs_clobbered_by_call(call_conv),
                opcode: Opcode::Call,
                caller_callconv: call_conv,
                callee_callconv: call_conv,
                callee_pop_size: 0,
            }),
        });
        insts
    }

    fn get_number_of_spillslots_for_value(
        rc: RegClass,
        _target_vector_bytes: u32,
        isa_flags: &RiscvFlags,
    ) -> u32 {
        // We allocate in terms of 8-byte slots.
        match rc {
            RegClass::Int => 1,
            RegClass::Float => 1,
            RegClass::Vector => (isa_flags.min_vec_reg_size() / 8) as u32,
        }
    }

    /// Get the current virtual-SP offset from an instruction-emission state.
    fn get_virtual_sp_offset_from_state(s: &EmitState) -> i64 {
        s.virtual_sp_offset
    }

    /// Get the nominal-SP-to-FP offset from an instruction-emission state.
    fn get_nominal_sp_to_fp(s: &EmitState) -> i64 {
        s.nominal_sp_to_fp
    }

    fn get_machine_env(_flags: &settings::Flags, _call_conv: isa::CallConv) -> &MachineEnv {
        static MACHINE_ENV: OnceLock<MachineEnv> = OnceLock::new();
        MACHINE_ENV.get_or_init(create_reg_enviroment)
    }

    fn get_regs_clobbered_by_call(call_conv_of_callee: isa::CallConv) -> PRegSet {
        if call_conv_of_callee == isa::CallConv::Tail {
            TAIL_CLOBBERS
        } else {
            DEFAULT_CLOBBERS
        }
    }

    fn compute_frame_layout(
        call_conv: isa::CallConv,
        flags: &settings::Flags,
        _sig: &Signature,
        regs: &[Writable<RealReg>],
        is_leaf: bool,
        stack_args_size: u32,
        fixed_frame_storage_size: u32,
        outgoing_args_size: u32,
    ) -> FrameLayout {
        let mut regs: Vec<Writable<RealReg>> = regs
            .iter()
            .cloned()
            .filter(|r| is_reg_saved_in_prologue(call_conv, r.to_reg()))
            .collect();

        regs.sort();

        // Compute clobber size.
        let clobber_size = compute_clobber_size(&regs);

        // Compute linkage frame size.
        let setup_area_size = if flags.preserve_frame_pointers()
            || !is_leaf
            // The function arguments that are passed on the stack are addressed
            // relative to the Frame Pointer.
            || stack_args_size > 0
            || clobber_size > 0
            || fixed_frame_storage_size > 0
        {
            16 // FP, LR
        } else {
            0
        };

        // Return FrameLayout structure.
        debug_assert!(outgoing_args_size == 0);
        FrameLayout {
            stack_args_size,
            setup_area_size,
            clobber_size,
            fixed_frame_storage_size,
            outgoing_args_size,
            clobbered_callee_saves: regs,
        }
    }

    fn gen_inline_probestack(
        insts: &mut SmallInstVec<Self::I>,
        call_conv: isa::CallConv,
        frame_size: u32,
        guard_size: u32,
    ) {
        // Unroll at most n consecutive probes, before falling back to using a loop
        const PROBE_MAX_UNROLL: u32 = 3;
        // Number of probes that we need to perform
        let probe_count = align_to(frame_size, guard_size) / guard_size;

        // Must be a caller-saved register that is not an argument.
        let tmp = match call_conv {
            isa::CallConv::Tail => Writable::from_reg(x_reg(1)),
            _ => Writable::from_reg(x_reg(28)), // t3
        };

        if probe_count <= PROBE_MAX_UNROLL {
            Self::gen_probestack_unroll(insts, tmp, guard_size, probe_count)
        } else {
            insts.push(Inst::StackProbeLoop {
                guard_size,
                probe_count,
                tmp,
            });
        }
    }
}

impl Riscv64ABICallSite {
    pub fn emit_return_call(mut self, ctx: &mut Lower<Inst>, args: isle::ValueSlice) {
        let (new_stack_arg_size, old_stack_arg_size) =
            self.emit_temporary_tail_call_frame(ctx, args);

        let dest = self.dest().clone();
        let opcode = self.opcode();
        let uses = self.take_uses();
        let info = Box::new(ReturnCallInfo {
            uses,
            opcode,
            old_stack_arg_size,
            new_stack_arg_size,
        });

        match dest {
            CallDest::ExtName(name, RelocDistance::Near) => {
                ctx.emit(Inst::ReturnCall {
                    callee: Box::new(name),
                    info,
                });
            }
            CallDest::ExtName(name, RelocDistance::Far) => {
                let callee = ctx.alloc_tmp(ir::types::I64).only_reg().unwrap();
                ctx.emit(Inst::LoadExtName {
                    rd: callee,
                    name: Box::new(name),
                    offset: 0,
                });
                ctx.emit(Inst::ReturnCallInd {
                    callee: callee.to_reg(),
                    info,
                });
            }
            CallDest::Reg(callee) => ctx.emit(Inst::ReturnCallInd { callee, info }),
        }
    }
}

const CALLEE_SAVE_X_REG: [bool; 32] = [
    false, false, true, false, false, false, false, false, // 0-7
    true, true, false, false, false, false, false, false, // 8-15
    false, false, true, true, true, true, true, true, // 16-23
    true, true, true, true, false, false, false, false, // 24-31
];
const CALLEE_SAVE_F_REG: [bool; 32] = [
    false, false, false, false, false, false, false, false, // 0-7
    true, false, false, false, false, false, false, false, // 8-15
    false, false, true, true, true, true, true, true, // 16-23
    true, true, true, true, false, false, false, false, // 24-31
];

/// This should be the registers that must be saved by callee.
#[inline]
fn is_reg_saved_in_prologue(conv: CallConv, reg: RealReg) -> bool {
    if conv == CallConv::Tail {
        return false;
    }

    match reg.class() {
        RegClass::Int => CALLEE_SAVE_X_REG[reg.hw_enc() as usize],
        RegClass::Float => CALLEE_SAVE_F_REG[reg.hw_enc() as usize],
        // All vector registers are caller saved.
        RegClass::Vector => false,
    }
}

fn compute_clobber_size(clobbers: &[Writable<RealReg>]) -> u32 {
    let mut clobbered_size = 0;
    for reg in clobbers {
        match reg.to_reg().class() {
            RegClass::Int => {
                clobbered_size += 8;
            }
            RegClass::Float => {
                clobbered_size += 8;
            }
            RegClass::Vector => unimplemented!("Vector Size Clobbered"),
        }
    }
    align_to(clobbered_size, 16)
}

const fn default_clobbers() -> PRegSet {
    PRegSet::empty()
        .with(px_reg(1))
        .with(px_reg(5))
        .with(px_reg(6))
        .with(px_reg(7))
        .with(px_reg(10))
        .with(px_reg(11))
        .with(px_reg(12))
        .with(px_reg(13))
        .with(px_reg(14))
        .with(px_reg(15))
        .with(px_reg(16))
        .with(px_reg(17))
        .with(px_reg(28))
        .with(px_reg(29))
        .with(px_reg(30))
        .with(px_reg(31))
        // F Regs
        .with(pf_reg(0))
        .with(pf_reg(1))
        .with(pf_reg(2))
        .with(pf_reg(3))
        .with(pf_reg(4))
        .with(pf_reg(5))
        .with(pf_reg(6))
        .with(pf_reg(7))
        .with(pf_reg(9))
        .with(pf_reg(10))
        .with(pf_reg(11))
        .with(pf_reg(12))
        .with(pf_reg(13))
        .with(pf_reg(14))
        .with(pf_reg(15))
        .with(pf_reg(16))
        .with(pf_reg(17))
        .with(pf_reg(28))
        .with(pf_reg(29))
        .with(pf_reg(30))
        .with(pf_reg(31))
        // V Regs - All vector regs get clobbered
        .with(pv_reg(0))
        .with(pv_reg(1))
        .with(pv_reg(2))
        .with(pv_reg(3))
        .with(pv_reg(4))
        .with(pv_reg(5))
        .with(pv_reg(6))
        .with(pv_reg(7))
        .with(pv_reg(8))
        .with(pv_reg(9))
        .with(pv_reg(10))
        .with(pv_reg(11))
        .with(pv_reg(12))
        .with(pv_reg(13))
        .with(pv_reg(14))
        .with(pv_reg(15))
        .with(pv_reg(16))
        .with(pv_reg(17))
        .with(pv_reg(18))
        .with(pv_reg(19))
        .with(pv_reg(20))
        .with(pv_reg(21))
        .with(pv_reg(22))
        .with(pv_reg(23))
        .with(pv_reg(24))
        .with(pv_reg(25))
        .with(pv_reg(26))
        .with(pv_reg(27))
        .with(pv_reg(28))
        .with(pv_reg(29))
        .with(pv_reg(30))
        .with(pv_reg(31))
}

const DEFAULT_CLOBBERS: PRegSet = default_clobbers();

// All allocatable registers are clobbered by calls using the `tail` calling
// convention.
const fn tail_clobbers() -> PRegSet {
    PRegSet::empty()
        // `x0` is the zero register, and not allocatable.
        .with(px_reg(1))
        // `x2` is the stack pointer, `x3` is the global pointer, and `x4` is
        // the thread pointer. None are allocatable.
        .with(px_reg(5))
        .with(px_reg(6))
        .with(px_reg(7))
        // `x8` is the frame pointer, and not allocatable.
        .with(px_reg(9))
        .with(px_reg(10))
        .with(px_reg(10))
        .with(px_reg(11))
        .with(px_reg(12))
        .with(px_reg(13))
        .with(px_reg(14))
        .with(px_reg(15))
        .with(px_reg(16))
        .with(px_reg(17))
        .with(px_reg(18))
        .with(px_reg(19))
        .with(px_reg(20))
        .with(px_reg(21))
        .with(px_reg(22))
        .with(px_reg(23))
        .with(px_reg(24))
        .with(px_reg(25))
        .with(px_reg(26))
        .with(px_reg(27))
        .with(px_reg(28))
        .with(px_reg(29))
        // `x30` and `x31` are reserved as scratch registers, and are not
        // allocatable.
        //
        // F Regs
        .with(pf_reg(0))
        .with(pf_reg(1))
        .with(pf_reg(2))
        .with(pf_reg(3))
        .with(pf_reg(4))
        .with(pf_reg(5))
        .with(pf_reg(6))
        .with(pf_reg(7))
        .with(pf_reg(9))
        .with(pf_reg(10))
        .with(pf_reg(11))
        .with(pf_reg(12))
        .with(pf_reg(13))
        .with(pf_reg(14))
        .with(pf_reg(15))
        .with(pf_reg(16))
        .with(pf_reg(17))
        .with(pf_reg(18))
        .with(pf_reg(19))
        .with(pf_reg(20))
        .with(pf_reg(21))
        .with(pf_reg(22))
        .with(pf_reg(23))
        .with(pf_reg(24))
        .with(pf_reg(25))
        .with(pf_reg(26))
        .with(pf_reg(27))
        .with(pf_reg(28))
        .with(pf_reg(29))
        .with(pf_reg(30))
        .with(pf_reg(31))
        // V Regs
        .with(pv_reg(0))
        .with(pv_reg(1))
        .with(pv_reg(2))
        .with(pv_reg(3))
        .with(pv_reg(4))
        .with(pv_reg(5))
        .with(pv_reg(6))
        .with(pv_reg(7))
        .with(pv_reg(8))
        .with(pv_reg(9))
        .with(pv_reg(10))
        .with(pv_reg(11))
        .with(pv_reg(12))
        .with(pv_reg(13))
        .with(pv_reg(14))
        .with(pv_reg(15))
        .with(pv_reg(16))
        .with(pv_reg(17))
        .with(pv_reg(18))
        .with(pv_reg(19))
        .with(pv_reg(20))
        .with(pv_reg(21))
        .with(pv_reg(22))
        .with(pv_reg(23))
        .with(pv_reg(24))
        .with(pv_reg(25))
        .with(pv_reg(26))
        .with(pv_reg(27))
        .with(pv_reg(28))
        .with(pv_reg(29))
        .with(pv_reg(30))
        .with(pv_reg(31))
}

const TAIL_CLOBBERS: PRegSet = tail_clobbers();

fn create_reg_enviroment() -> MachineEnv {
    // Some C Extension instructions can only use a subset of the registers.
    // x8 - x15, f8 - f15, v8 - v15 so we should prefer to use those since
    // they allow us to emit C instructions more often.
    //
    // In general the order of preference is:
    //   1. Compressible Caller Saved registers.
    //   2. Non-Compressible Caller Saved registers.
    //   3. Compressible Callee Saved registers.
    //   4. Non-Compressible Callee Saved registers.

    let preferred_regs_by_class: [Vec<PReg>; 3] = {
        let x_registers: Vec<PReg> = (10..=15).map(px_reg).collect();
        let f_registers: Vec<PReg> = (10..=15).map(pf_reg).collect();
        let v_registers: Vec<PReg> = (8..=15).map(pv_reg).collect();

        [x_registers, f_registers, v_registers]
    };

    let non_preferred_regs_by_class: [Vec<PReg>; 3] = {
        // x0 - x4 are special registers, so we don't want to use them.
        // Omit x30 and x31 since they are the spilltmp registers.

        // Start with the Non-Compressible Caller Saved registers.
        let x_registers: Vec<PReg> = (5..=7)
            .chain(16..=17)
            .chain(28..=29)
            // The first Callee Saved register is x9 since its Compressible
            // Omit x8 since it's the frame pointer.
            .chain(9..=9)
            // The rest of the Callee Saved registers are Non-Compressible
            .chain(18..=27)
            .map(px_reg)
            .collect();

        // Prefer Caller Saved registers.
        let f_registers: Vec<PReg> = (0..=7)
            .chain(16..=17)
            .chain(28..=31)
            // Once those are exhausted, we should prefer f8 and f9 since they are
            // callee saved, but compressible.
            .chain(8..=9)
            .chain(18..=27)
            .map(pf_reg)
            .collect();

        let v_registers = (0..=7).chain(16..=31).map(pv_reg).collect();

        [x_registers, f_registers, v_registers]
    };

    MachineEnv {
        preferred_regs_by_class,
        non_preferred_regs_by_class,
        fixed_stack_slots: vec![],
        scratch_by_class: [None, None, None],
    }
}

impl Riscv64MachineDeps {
    fn gen_probestack_unroll(
        insts: &mut SmallInstVec<Inst>,
        tmp: Writable<Reg>,
        guard_size: u32,
        probe_count: u32,
    ) {
        // When manually unrolling adjust the stack pointer and then write a zero
        // to the stack at that offset.
        //
        // We do this because valgrind expects us to never write beyond the stack
        // pointer and associated redzone.
        // See: https://github.com/bytecodealliance/wasmtime/issues/7454

        // Store the adjust amount in a register upfront, so we don't have to
        // reload it for each probe. It's worth loading this as a negative and
        // using an `add` instruction since we have compressed versions of `add`
        // but not the `sub` instruction.
        insts.extend(Inst::load_constant_u64(tmp, (-(guard_size as i64)) as u64));

        for _ in 0..probe_count {
            insts.push(Inst::AluRRR {
                alu_op: AluOPRRR::Add,
                rd: writable_stack_reg(),
                rs1: stack_reg(),
                rs2: tmp.to_reg(),
            });

            insts.push(Self::gen_store_stack(
                StackAMode::SPOffset(0, I8),
                zero_reg(),
                I32,
            ));
        }

        // Restore the stack pointer to its original value
        insts.extend(Self::gen_sp_reg_adjust((guard_size * probe_count) as i32));
    }
}
