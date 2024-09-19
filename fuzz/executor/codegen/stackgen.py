import os
import sys

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(source_dir)
sys.path[0] = root_dir

import config
from codegen.generator import *
from codegen.rng import ConsumeRng, RandomRng
from codegen import components

class StackExtRuleProvider(ExtRuleProvider):
    def __init__(self):
        super().__init__()

        # control instructions
        if not 'noout' in config.codegen_blacklist_types:
            ExtRuleProvider.add_rule((), (), [Instr('nop', ())], self.typing_rules)

        # table instructions
        for op in TABLE_OPCODES:
            if op in BLOCKED_OPCODES:
                continue
            
            inst_data = INSTRUCTIONS[op]
            assert len(inst_data.output_types) <= 1

            param_types = tuple([x if x != 'any' else 'funcref' for x in inst_data.input_types])
            if 'funcref' in config.codegen_blacklist_types and 'funcref' in param_types:
                continue

            if len(inst_data.output_types) == 0:
                if 'noout' in config.codegen_blacklist_types:
                    continue
                ret_types = ()
            elif inst_data.output_types[0] == 'any':
                if 'funcref' in config.codegen_blacklist_types:
                    continue
                ret_types = ('funcref',)
            else:
                ret_types = inst_data.output_types
            
            instrs = []
            for _ in param_types:
                instrs.append(Instr('arg', (len(instrs),))) # arguments
            instrs.append(Instr(op, tuple([('oparg', idx) for idx, _ in enumerate(inst_data.operands)])))
            
            ExtRuleProvider.add_rule(param_types, ret_types, instrs, self.typing_rules)
        
        # reference instruction
        # no ref.null, ref.func -> they are generated manually at argconst
        ref_is_null_inst = INSTRUCTIONS['ref.is_null']
        instrs = [Instr('arg', (0,))]
        if not 'funcref' in config.codegen_blacklist_types:
            param_types = ('funcref',)
            ret_types = ref_is_null_inst.output_types
            new_instrs = instrs+[Instr('ref.is_null', ())]
            ExtRuleProvider.add_rule(param_types, ret_types, new_instrs, self.typing_rules)
        if not 'externref' in config.codegen_blacklist_types:
            param_types = ('externref',)
            ret_types = ref_is_null_inst.output_types
            new_instrs = instrs+[Instr('ref.is_null', ())]
            ExtRuleProvider.add_rule(param_types, ret_types, new_instrs, self.typing_rules)


class StackRecord():
    def __init__(self, stack, instr_node):
        self.stack = stack.copy()
        self.instr_node = instr_node


class StackGenBlockContext(GenContext):
    def __init__(self, rng, globalctx, ctx_stack, struct_depth=0):
        super().__init__(rng)
        self.globalctx = globalctx
        self.ctx_stack = ctx_stack + [self] # stack of contexts for br and return
        assert type(struct_depth) == int
        self.struct_depth = struct_depth

        # type info
        self.params = None
        self.rets = []

        # generation
        self.stack = []
        self.records_rev = []

        self.init_done = False
        self.gen_done = False
    
    def init_stack(self, first_stack):
        assert not self.init_done and not self.gen_done
        self.stack = first_stack.copy()
        self.rets = first_stack.copy()
        self.init_done = True
    
    def set_target_params(self, params):
        assert self.init_done and not self.gen_done and self.params == None
        self.params = params.copy()
    
    def get_stack_goal(self): # needs override
        return self.rets
    
    def get_struct_stack(self):
        struct_rets = []
        while len(self.stack) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_struct_ret): # default is 'noout'
            struct_rets.append(self.stack.pop())
        struct_rets = struct_rets[::-1] # should be reversed!
        return struct_rets

    def get_block_type(self):
        assert self.init_done and self.gen_done
        assert self.params != None
        if len(self.params) == 0 and len(self.rets) == 0: # emptyblock
            return 'emptyblock'
        elif len(self.params) == 0 and len(self.rets) == 1:
            return self.rets[0]
        else:
            return self.globalctx.alloc_type_section(self.params, self.rets)

    def generate(self, rule_provider):
        assert self.init_done and not self.gen_done

        while not self.gen_done:
            # try finish
            if not self.rng.get_choice_prob(1 - config.codegen_stackgen_prob_struct_exit): # default is exit
                self.gen_done = True
                continue

            # continue
            if self.struct_depth + len(self.ctx_stack) < config.codegen_stackgen_maximum_depth and \
                self.rng.get_choice_prob(config.codegen_stackgen_prob_struct_gen): # struct gen (default: false)
                start_stack = self.stack.copy()
                
                struct_list = ('block', 'loop', 'if')
                if self.rng.get_choice_prob(config.codegen_stackgen_prob_unreachable):
                    struct_choice = 'unreachable'
                elif self.rng.get_choice_prob(config.codegen_stackgen_prob_call):
                    struct_choice = 'call'
                else:
                    struct_choice = self.rng.get_choice_arr(struct_list)
                
                if struct_choice == 'block':
                    struct_rets = self.get_struct_stack()
                    block_ctx = StackGenBlockContext(self.rng, self.globalctx, self.ctx_stack)
                    block_ctx.init_stack(struct_rets)
                    block_ctx.generate(rule_provider)
                    assert block_ctx.gen_done
                    blocktype = block_ctx.get_block_type()

                    self.records_rev.append(StackRecord(start_stack, InstrNode.from_single_instr(Instr('end', ()))))
                    self.records_rev += block_ctx.records_rev
                    self.stack += block_ctx.params
                    self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('block', (blocktype,)))))

                elif struct_choice == 'loop':
                    struct_rets = self.get_struct_stack()
                    loop_ctx = StackGenLoopContext(self.rng, self.globalctx, self.ctx_stack)
                    loop_ctx.init_stack(struct_rets)
                    loop_ctx.generate(rule_provider)
                    assert loop_ctx.gen_done
                    blocktype = loop_ctx.get_block_type()

                    self.records_rev.append(StackRecord(start_stack, InstrNode.from_single_instr(Instr('end', ()))))
                    self.records_rev += loop_ctx.records_rev
                    self.stack += loop_ctx.params
                    self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('loop', (blocktype,)))))

                elif struct_choice == 'if':
                    struct_rets = self.get_struct_stack()
                    if_ctx = StackGenBlockContext(self.rng, self.globalctx, self.ctx_stack)
                    if_ctx.init_stack(struct_rets)
                    if_ctx.generate(rule_provider)
                    assert if_ctx.gen_done
                    blocktype = if_ctx.get_block_type()

                    if len(if_ctx.params) == 0 and len(if_ctx.rets) == 0 and \
                        self.rng.get_choice_prob(config.codegen_stackgen_prob_struct_skipelse): # skip else
                        self.records_rev.append(StackRecord(start_stack, InstrNode.from_single_instr(Instr('end', ()))))
                        self.records_rev += if_ctx.records_rev
                        self.stack += if_ctx.params
                        self.stack.append('i32')
                        self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('if', (blocktype,)))))
                    else:
                        else_ctx = StackGenBlockContext(self.rng, self.globalctx, self.ctx_stack)
                        else_ctx.init_stack(struct_rets)
                        else_ctx.set_target_params(if_ctx.params)
                        else_ctx.generate(rule_provider)
                        assert else_ctx.gen_done

                        self.records_rev.append(StackRecord(start_stack, InstrNode.from_single_instr(Instr('end', ()))))
                        self.records_rev += else_ctx.records_rev
                        self.records_rev.append(StackRecord(start_stack, InstrNode.from_single_instr(Instr('else', ()))))
                        self.records_rev += if_ctx.records_rev
                        self.stack += if_ctx.params
                        self.stack.append('i32')
                        self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('if', (blocktype,)))))
                
                elif struct_choice == 'unreachable':
                    struct_rets = self.get_struct_stack()
                    allowed_types = list(filter(lambda x: x != 'noout', ALLOWED_TYPES))
                    while self.rng.get_choice_prob(config.codegen_stackgen_prob_multiret):
                        self.stack.append(self.rng.get_choice_arr(allowed_types))
                    self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('unreachable', ()))))

                elif struct_choice == 'call':
                    # check if there are functions that may be reused
                    func_candidates = []
                    for idx, func in enumerate(self.globalctx.funcs):
                        if not func.gen_done:
                            continue
                        
                        if len(func.rets) == 0 or func.rets == self.stack[-len(func.rets):]:
                            func_candidates.append((idx, func))

                    if len(func_candidates) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_reuse_func):
                        func_idx, func = self.rng.get_choice_arr(func_candidates)
                        func_params = func.args
                        if len(func.rets) > 0:
                            self.stack = self.stack[:-len(func.rets)]
                        if not self.rng.get_choice_prob(config.codegen_stackgen_prob_call_indirect): # call
                            node = InstrNode(func_params, func.rets, [Instr('call', (func_idx,))])
                        else: # call_indirect
                            rand_table_off = self.rng.get_choice(MAX_TABLE_PARAMS) # ignoring table params...
                            instrs = []
                            # instrs.append(Instr('i32.const', (rand_table_off,)))
                            # instrs.append(Instr('ref.func', (func_idx,)))
                            # instrs.append(Instr('table.set', (0,)))
                            instrs.append(Instr('i32.const', (rand_table_off,)))
                            instrs.append(Instr('call_indirect', (f'func{func_idx}', 0)))
                            node = InstrNode(func_params, func.rets, instrs)
                        self.records_rev.append(StackRecord(self.stack, node))
                        self.stack += func_params
                    else:
                        struct_rets = self.get_struct_stack()
                        new_func_idx = self.globalctx.generate_func(rule_provider, first_stack=struct_rets, 
                            struct_depth=self.struct_depth + len(self.ctx_stack))
                        new_func_params = self.globalctx.funcs[new_func_idx].args
                        node = InstrNode(new_func_params, struct_rets, [Instr('call', (new_func_idx,))])
                        self.records_rev.append(StackRecord(self.stack, node))
                        self.stack += new_func_params
                
            else: # extension of types
                exit_candidates = []
                for idx, prev_ctx in enumerate(self.ctx_stack):
                    prev_stack = prev_ctx.get_stack_goal()
                    if prev_stack == None or prev_stack == self.stack: # length should be same
                        exit_candidates.append((idx, prev_ctx))

                if len(exit_candidates) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_br_if):
                    exit_idx, exit_ctx = self.rng.get_choice_arr(exit_candidates)
                    self.gen_br_if(exit_idx, exit_ctx)

                elif len(exit_candidates) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_br):
                    self.gen_br(exit_candidates)
                else:
                    self.extend_type(rule_provider)
        
        if self.params == None:
            self.params = self.stack
        else:
            self.massage_types(self.params)
    
    def extend_type(self, rule_provider: RuleProvider):
        assert self.init_done and not self.gen_done
        if len(self.stack) == 0:
            ty = 'noout'
        elif self.rng.get_choice_prob(config.codegen_stackgen_prob_stack_pop):
            ty = self.stack.pop()
        else:
            ty = 'noout'
        
        if not self.rng.get_choice_prob(config.codegen_stackgen_prob_argconst + config.codegen_stackgen_prob_var_gen):
            chosen_rule = rule_provider.get_rule(ty, self.rng)
            assert (len(chosen_rule.ret_types) == 0 and ty == 'noout') or (len(chosen_rule.ret_types) == 1 and chosen_rule.ret_types[0] == ty)

            # scan through the rule, count number of arg uses
            args_use_cnt = [0] * len(chosen_rule.param_types)
            for instr in chosen_rule.instrs:
                if instr.name == 'arg':
                    assert len(instr.operands) == 1
                    arg_idx = instr.operands[0]
                    assert arg_idx < len(chosen_rule.param_types)
                    args_use_cnt[arg_idx] += 1
            
            # optimize local generation: skip gen if redundant
            opted_instrs = []
            local_sets = list(range(len(chosen_rule.param_types) - 1, -1, -1))
            for idx, instr in enumerate(chosen_rule.instrs):
                if instr.name == "arg":
                    assert len(instr.operands) == 1
                    arg_idx = instr.operands[0]
                    assert arg_idx < len(chosen_rule.param_types)

                    if args_use_cnt[arg_idx] == 1 and local_sets[-1] == arg_idx:
                        local_sets.pop()
                        continue
                    else:
                        assert args_use_cnt[arg_idx] > 0
                        opted_instrs = chosen_rule.instrs[idx:]
                        break
                else:
                    opted_instrs = chosen_rule.instrs[idx:]
                    break
            
            # generate local sets
            new_instrs = [] # may be empty (there is one case that this is empty)
            args_store = {} # stores generated args
            for param_idx in local_sets:
                param_ty = chosen_rule.param_types[param_idx]
                local_instrs_tuple = self.ctx_stack[0].alloc_local(param_ty)
                new_instrs.append(local_instrs_tuple[0])
                args_store[param_idx] = local_instrs_tuple[1]

            # gen args and operands
            oparg_store = {} # stores generated opargs
            for instr in opted_instrs:
                if instr.name == "arg":
                    assert len(instr.operands) == 1
                    arg_idx = instr.operands[0]
                    assert arg_idx < len(chosen_rule.param_types)

                    new_instrs.append(args_store[arg_idx])

                elif len(instr.operands) > 0 and \
                        any(oper[0] == "oparg" for oper in instr.operands if type(oper) is tuple):
                    new_operands = []
                    for oper_idx, oper in enumerate(instr.operands):
                        if type(oper) is tuple and oper[0] == "oparg":
                            assert len(oper) == 2
                            oparg_idx = oper[1]
                            if oparg_idx in oparg_store:
                                new_operands.append(oparg_store[oparg_idx])
                            else:
                                instr_info = INSTRUCTIONS[instr.name]
                                assert oper_idx < len(instr_info.operands)
                                new_operand = self.gen_operand(instr.name, instr_info.operands[oper_idx], oper_idx, oparg_store, chosen_rule.conds)
                                # oparg_store[oparg_idx] = new_operand
                                new_operands.append(new_operand)
                        else:
                            instr_info = INSTRUCTIONS[instr.name]
                            assert oper_idx < len(instr_info.operands)
                            new_operand = self.perturb_operand(instr.name, instr_info.operands[oper_idx], oper)
                            new_operands.append(new_operand)
                    assert len(new_operands) > 0
                    new_instrs.append(Instr(instr.name, tuple(new_operands)))

                else:
                    new_instrs.append(instr)

            # canonicalize nans
            if len(new_instrs) > 0 and self.canonicalize_nans and new_instrs[-1].name in FLOAT_CANON_OPCODES:
                ty_name = new_instrs[-1].name.split('.')[0]
                new_instrs += self.ctx_stack[0].get_canonicalization_nans_instrs(ty_name)
            
            self.records_rev.append(StackRecord(self.stack, InstrNode(chosen_rule.param_types, chosen_rule.ret_types, new_instrs))) # later processed
            self.stack += chosen_rule.param_types
        elif ty == 'noout' or self.rng.get_choice_prob(config.codegen_stackgen_prob_var_gen /
                (config.codegen_stackgen_prob_var_gen + config.codegen_stackgen_prob_argconst)):
            self.gen_var(ty)
        else:
            self.gen_argconst(ty)
    
    # global or local
    def gen_var(self, ty): # ty being already popped
        assert self.init_done
        if ty == 'noout': # set
            if self.rng.get_choice_prob(config.codegen_stackgen_prob_globalgen):
                new_type_candidates = list(filter(lambda x: x != 'noout', ALLOWED_TYPES))
                new_type = self.rng.get_choice_arr(new_type_candidates)
                set_instr = self.globalctx.alloc_global(new_type, allow_dup=True)[0]
                node = InstrNode((new_type,), (), [set_instr])
                self.stack.append(new_type)
                self.records_rev.append(StackRecord(self.stack, node))
            else:
                new_type_candidates = list(filter(lambda x: x != 'noout', ALLOWED_TYPES))
                new_type = self.rng.get_choice_arr(new_type_candidates)
                set_instr = self.ctx_stack[0].alloc_local(new_type, allow_dup=True)[0]
                node = InstrNode((new_type,), (), [set_instr])
                self.stack.append(new_type)
                self.records_rev.append(StackRecord(self.stack, node))
        else: # get, tee
            if self.rng.get_choice_prob(config.codegen_stackgen_prob_globalgen):
                get_instr = self.globalctx.alloc_global(ty, allow_dup=True)[1]
                node = InstrNode((), (ty,), [get_instr])
                self.records_rev.append(StackRecord(self.stack, node))
            elif self.rng.get_choice_prob(0.5): # tee
                tee_instr = self.ctx_stack[0].alloc_local(ty, allow_dup=True)[2]
                node = InstrNode((ty,), (ty,), [tee_instr])
                self.stack.append(ty)
                self.records_rev.append(StackRecord(self.stack, node))
            else:
                get_instr = self.ctx_stack[0].alloc_local(ty, allow_dup=True)[1]
                node = InstrNode((), (ty,), [get_instr])
                self.records_rev.append(StackRecord(self.stack, node))


    def gen_argconst(self, ty): # ty being already popped
        assert self.init_done
        assert ty != 'noout'
        if (self.ctx_stack[0].is_main and ty in ('funcref', 'externref')) or \
            self.rng.get_choice_prob(config.codegen_stackgen_prob_constgen):
            if not ty in ('funcref', 'externref'):
                opcode = f'{ty}.const'
                assert len(INSTRUCTIONS[opcode].operands) == 1
                const_operand = self.gen_operand(opcode, INSTRUCTIONS[opcode].operands[0], 0, {}, {})
                node = InstrNode((), (ty,), [Instr(opcode, (const_operand,))])
                self.records_rev.append(StackRecord(self.stack, node))
            elif ty == 'funcref':
                node = InstrNode((), ('funcref',), [Instr('ref.null', ('funcref',))])
                self.records_rev.append(StackRecord(self.stack, node))
                
                # select from funcs, 0 is null
                # choice = self.rng.get_choice(len(self.globalctx.funcs) + 1)
                # if choice == 0:
                #     node = InstrNode((), ('funcref',), [Instr('ref.null', ('funcref',))])
                #     self.records_rev.append(StackRecord(self.stack, node))
                # else: # impossible without element segments
                #     node = InstrNode((), ('funcref',), [Instr('ref.func', (choice - 1,))])
                #     self.records_rev.append(StackRecord(self.stack, node))
            else: # externref
                node = InstrNode((), ('externref',), [Instr('ref.null', ('externref',))])
                self.records_rev.append(StackRecord(self.stack, node))
        else:
            arg_instrs = self.ctx_stack[0].alloc_arg(ty)
            node = InstrNode((), (ty,), arg_instrs)
            self.records_rev.append(StackRecord(self.stack, node))

    def massage_types(self, target_stack_state):
        assert self.init_done and self.gen_done
        stack_intersect = []
        for i, j in zip(self.stack, target_stack_state):
            if i == j:
                stack_intersect.append(i)
            else:
                break
        
        # drop from stack with argconsts
        while len(self.stack) > len(stack_intersect):
            ty = self.stack.pop()
            self.gen_argconst(ty)
        
        # add to stack with local.set - these are later moved to memory at the end of the function
        for ty in target_stack_state[len(stack_intersect):]:
            local_instrs = self.ctx_stack[0].alloc_local(ty)
            set_local = local_instrs[0]
            get_local = local_instrs[1]
            if ty in ('funcref', 'externref'):
                pre_instrs, post_instrs = self.globalctx.alloc_table_param(ty, False)
            else:
                pre_instrs, post_instrs = self.globalctx.alloc_mem_param(ty, False)

            # temp save to local
            self.records_rev.append(StackRecord(self.stack, InstrNode((), (ty,), [set_local])))
            self.stack.append(ty)

            # move local value to memory or table at the end of the block
            self.records_rev.insert(0, StackRecord(self.rets, InstrNode((), (), pre_instrs + [get_local] + post_instrs)))

    def gen_br_if(self, exit_idx, exit_ctx):
        if exit_ctx.get_stack_goal() == None: # loop needs to match with the params
            exit_ctx.set_target_params(self.stack)
        
        labelidx = len(self.ctx_stack) - 1 - exit_idx
        self.stack.append('i32')
        self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('br_if', (labelidx,)))))

    def gen_br(self, exit_candidates):
        exit_opcodes = ['br', 'br_table']
        if 0 in [x[0] for x in exit_candidates]:
            exit_opcodes.append('return')

        exit_opcode = self.rng.get_choice_arr(exit_opcodes)
        if exit_opcode == 'return':
            exit_idx, exit_ctx = next(filter(lambda x: x[0] == 0, exit_candidates))
        else:
            exit_idx, exit_ctx = self.rng.get_choice_arr(exit_candidates)
        
        if exit_ctx.get_stack_goal() == None: # loop needs to match with the params
            self.get_new_stack()
            exit_ctx.set_target_params(self.stack)
        else:
            self.stack = exit_ctx.get_stack_goal().copy()

        labelidx = len(self.ctx_stack) - 1 - exit_idx
        if exit_opcode == 'br':
            self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('br', (labelidx,)))))
        elif exit_opcode == 'br_table':
            # construct labelidx list
            exitidx_list = [exit_idx]
            new_candidates = []
            for exit_idx, exit_ctx in exit_candidates:
                target_goal = exit_ctx.get_stack_goal()
                if target_goal == None or target_goal == self.stack:
                    new_candidates.append((exit_idx, exit_ctx))
            
            while len(new_candidates) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_br_table_conti):
                exit_idx, exit_ctx = self.rng.get_choice_arr(new_candidates)
                if exit_ctx.get_stack_goal() == None: # loop needs to match with the params
                    exit_ctx.set_target_params(self.stack)
                exitidx_list.append(exit_idx)
            labelidx_list = [len(self.ctx_stack) - 1 - x for x in exitidx_list]

            self.stack.append('i32')
            self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(
                Instr('br_table', (labelidx_list[:-1], labelidx_list[-1])))))
        else: # return
            self.records_rev.append(StackRecord(self.stack, InstrNode.from_single_instr(Instr('return', ()))))
    
    def get_instrs(self):
        assert self.init_done and self.gen_done

        result_instrs = []
        records = self.records_rev[::-1]
        for record in records:
            result_instrs += record.instr_node.instrs
        
        return result_instrs
    
    def get_compat_instrs(self):
        instrs = self.get_instrs()
        compat_instrs = []
        for instr in instrs:
            if instr.name in ("local.set", "local.get", "local.tee"):
                compat_instrs.append((instr.name, self.ctx_stack[0].get_local_idx(instr.operands[0])))
            elif instr.name == "call_indirect":
                compat_instrs.append((
                    instr.name,
                    len(self.globalctx.types) + int(instr.operands[0][len('func'):]),
                    instr.operands[1]
                ))
            else:
                compat_instrs.append((instr.name,)+instr.operands)
        return compat_instrs

    def get_new_stack(self):
        # used for unconstrained control transfer
        new_stack = []
        allowed_types = list(filter(lambda x: x != 'noout', ALLOWED_TYPES))
        while self.rng.get_choice_prob(config.codegen_stackgen_prob_multiret):
            new_stack.append(self.rng.get_choice_arr(allowed_types))
        self.stack = new_stack

    @override
    def gen_operand(self, opcode, operand, oparg_idx, oparg_store, conds):
        if operand == 'funcidx':
            new_operand = self.rng.get_choice(len(self.globalctx.funcs)) # TODO: may create function here
            oparg_store[oparg_idx] = new_operand
            return new_operand
        else:
            return super().gen_operand(opcode, operand, oparg_idx, oparg_store, conds)


class StackGenLoopContext(StackGenBlockContext):
    def __init__(self, rng, globalctx, ctx_stack):
        super().__init__(rng, globalctx, ctx_stack)

    @override
    def get_stack_goal(self):
        return self.params


class StackGenFuncContext(StackGenBlockContext):
    def __init__(self, rng, globalctx, struct_depth=0, is_main=False):
        super().__init__(rng, globalctx, [], struct_depth)

        self.args = []
        self.locals = []
        self.is_main = is_main

    @override
    def init_stack(self, first_stack=None):
        assert not self.init_done and not self.gen_done
        
        if first_stack == None:
            frame = []
            allowed_types = ALLOWED_TYPES.copy()
            if self.is_main:
                allowed_types = list(filter(lambda x: x != 'funcref' and x != 'externref', allowed_types))
            ty_choice_pool = list(filter(lambda x: x != 'noout', allowed_types))
            ty = self.rng.get_choice_arr(ty_choice_pool)
            frame.append(ty)
            while self.rng.get_choice_prob(config.codegen_stackgen_prob_multiret):
                ty = self.rng.get_choice_arr(ty_choice_pool)
                frame.append(ty)
            assert len(frame) > 0
        else:
            frame = first_stack
        
        super().init_stack(frame)

    def alloc_arg(self, ty, allow_dup=True):
        assert ty != 'noout' and ty in ALLOWED_TYPES
        
        if allow_dup:
            arg_choice_list = [idx for idx, arg_ty in enumerate(self.args) if arg_ty == ty]
            arg_choice_list.append(len(self.args))
            arg_choice = self.rng.get_choice_arr(arg_choice_list)
        else:
            arg_choice = len(self.args)
        
        if arg_choice == len(self.args):
            self.args.append(ty)
        
        return [Instr('local.get', (f"arg{arg_choice}",))]

    def alloc_local(self, ty, allow_dup=True):
        assert ty != 'noout' and ty in ALLOWED_TYPES

        local_choice_list = [idx for idx, local_ty in enumerate(self.locals) if local_ty == ty]
        if allow_dup and len(local_choice_list) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_reuse_local):
            local_choice = self.rng.get_choice_arr(local_choice_list)
        else:
            local_choice = len(self.locals)
            self.locals.append(ty)

        return [
            Instr('local.set', (f"local{local_choice}",)),
            Instr('local.get', (f"local{local_choice}",)),
            Instr('local.tee', (f"local{local_choice}",)),
        ]

    def get_local_idx(self, opname):
        if opname.startswith("arg"):
            return int(opname[3:])
        elif opname.startswith("local"):
            return int(opname[5:]) + len(self.args) # WARNING: ty_name doesn't use v128 as the name
    
    def get_canonicalization_nans_instrs(self, ty_name):
        assert ty_name in ('f32', 'f64', 'f32x4', 'f64x2')

        # [Mechanism] (from wasm-smith)
        # f32.add -> target instruction
        # local.tee <idx> (select - true)
        # const(+nan)     (select - false)
        # local.get <idx>
        # local.get <idx>
        # eq (if at least one side is nan, returns 0)
        # select

        instrs = []
        locals_instr_tuple = self.alloc_local(ty_name if ty_name in ('f32', 'f64') else 'v128')
        instrs.append(locals_instr_tuple[2]) # local.tee

        # const(+nan)
        if ty_name in ('f32', 'f64'):
            instrs.append(Instr(f'{ty_name}.const', (float('nan'),)))
        elif ty_name == 'f32x4':
            nan = 0x7fc000007fc000007fc000007fc00000
            instrs.append(Instr(f'v128.const', (nan,)))
        else: # f64x2
            nan = 0x7ff80000000000007ff8000000000000
            instrs.append(Instr(f'v128.const', (nan,)))

        instrs.append(locals_instr_tuple[1]) # local.get
        instrs.append(locals_instr_tuple[1]) # local.get
        instrs.append(Instr(f'{ty_name}.eq', ()))

        if ty_name in ('f32', 'f64'):
            instrs.append(Instr(f'select', ()))
        else:
            instrs.append(Instr(f'v128.bitselect', ()))

        return instrs

    def gen_call_adapter(self, args, rets, func_idx):
        self.init_done = True

        # get rets
        arg_gets = []
        arg_sets = []
        for ret in rets:
            local_instrs = self.alloc_local(ret, False)

            # get
            if ret != 'v128':
                arg_gets.append(local_instrs[1])
                self.rets.append(ret)
            else:
                arg_gets.append(local_instrs[1])
                arg_gets.append(Instr('i64x2.extract_lane', (1,))) # high bytes first
                arg_gets.append(local_instrs[1])
                arg_gets.append(Instr('i64x2.extract_lane', (0,))) # low bytes next
                self.rets += ['i64', 'i64']
            
            # set
            arg_sets.append(local_instrs[0])
        
        instrs = arg_sets[::-1] + arg_gets
        node = InstrNode(rets, self.rets, instrs)
        self.records_rev.append(StackRecord(self.rets, node))

        # call
        node = InstrNode(args, rets, [Instr('call', (func_idx,))])
        self.records_rev.append(StackRecord(self.stack, node))

        # set args
        for arg in args[::-1]:
            if arg != "v128":
                node = InstrNode([], [arg], [self.alloc_arg(arg, False)[0]])
            else:
                instrs = [Instr('v128.const', (0,))]
                instrs.append(self.alloc_arg('i64', False)[0])
                instrs.append(Instr('i64x2.replace_lane', (1,))) # high bytes first
                instrs.append(self.alloc_arg('i64', False)[0])
                instrs.append(Instr('i64x2.replace_lane', (0,))) # low bytes next
                node = InstrNode([], ['v128'], instrs)
            self.records_rev.append(StackRecord(self.stack, node))

        self.gen_done = True


class StackGenContext(GenContext):
    def __init__(self, rng):
        super().__init__(rng)

        self.funcs = [] # list of StackGenFuncContext
        self.types = [] # list of types: (params, rets)
        self.mem_params = [] # list of MemParam
        self.table_params = [] # list of TableParam
        self.globals = [] # list of globals - (ty, InstrNode)

        self.args = [] # args of main func
        self.rets = [] # rets of main func
        self.instrs = [] # instrs of main func
    
    def generate(self, rule_provider):
        if not config.engine_name in ("wasmtime", "wasmer", "wasmedge", "wamr"):
            main_func = StackGenFuncContext(self.rng, self, is_main=True)
            self.funcs.append(main_func)

            self.generate_func(rule_provider, first_stack=None, is_main=True)
            self.funcs[1].is_main = False
            
            assert len(self.funcs) > 1
            main_func.gen_call_adapter(self.funcs[1].args, self.funcs[1].rets, 1)
            
            self.args = self.funcs[1].args[::-1] # reverse since arguments are put in reverse in the adapter
            self.rets = self.funcs[1].rets
            self.instrs = self.funcs[1].get_instrs()
        else:
            self.generate_func(rule_provider, first_stack=None, is_main=True)

            self.args = self.funcs[0].args
            self.rets = self.funcs[0].rets
            self.instrs = self.funcs[0].get_instrs()
        assert len(self.funcs) > 0

    def generate_func(self, rule_provider, first_stack=None, is_main=False, struct_depth=0):
        new_func = StackGenFuncContext(self.rng, self, struct_depth=struct_depth, is_main=is_main)
        new_func.init_stack(first_stack)
        new_func.set_target_params([])
        self.funcs.append(new_func)
        new_func_idx = len(self.funcs) - 1
        new_func.generate(rule_provider)
        assert new_func.gen_done
        return new_func_idx
    
    def alloc_global(self, ty, allow_dup=True):
        assert ty != 'noout' and ty in ALLOWED_TYPES

        global_choice_list = [idx for idx, global_info in enumerate(self.globals) if global_info[0] == ty]
        if allow_dup and len(global_choice_list) > 0 and self.rng.get_choice_prob(config.codegen_stackgen_prob_reuse_global):
            global_choice = self.rng.get_choice_arr(global_choice_list)
        else:

            if not ty in ('funcref', 'externref'):
                opcode = f'{ty}.const'
                assert len(INSTRUCTIONS[opcode].operands) == 1
                const_operand = self.gen_operand(opcode, INSTRUCTIONS[opcode].operands[0], 0, {}, {})
                node = InstrNode((), (ty,), [Instr(opcode, (const_operand,))])
            elif ty == 'funcref':
                node = InstrNode((), ('funcref',), [Instr('ref.null', ('funcref',))])
                # # select from funcs, 0 is null
                # choice = self.rng.get_choice(len(self.funcs) + 1)
                # if choice == 0:
                #     node = InstrNode((), ('funcref',), [Instr('ref.null', ('funcref',))])
                # else:
                #     node = InstrNode((), ('funcref',), [Instr('ref.func', (choice - 1,))])
            else: # externref
                node = InstrNode((), ('funcref',), [Instr('ref.null', ('externref',))])

            global_choice = len(self.globals)
            self.globals.append((ty, node))

        return [
            Instr('global.set', (global_choice,)),
            Instr('global.get', (global_choice,)),
        ]

    def alloc_mem_param(self, ty, is_load):
        if len(self.mem_params) > MAX_MEM_PARAMS - 1:
            mem_loc_choice = self.rng.get_choice(MAX_MEM_PARAMS)
        else:
            mem_loc_choice = len(self.mem_params)
        mem_loc = mem_loc_choice * 0x10 + MEM_PARAMS_START
        if mem_loc_choice == len(self.mem_params):
            self.mem_params.append(MemParam(ty, mem_loc, is_load))
        
        instr_seq = []
        instr_seq.append(Instr('i32.const', (0,)))
        if is_load:
            instr_seq.append(Instr(f'{ty}.load', ((1, mem_loc),)))
            return instr_seq, []
        else:
            pre_instr_seq = instr_seq
            post_instr_seq = []
            post_instr_seq.append(Instr(f'{ty}.store', ((1, mem_loc),)))
            return pre_instr_seq, post_instr_seq
        
    def alloc_table_param(self, ty, is_load):
        if ty == 'externref':
            if is_load:
                return [Instr('ref.null', ('externref',))], []
            else:
                # cannot save externref to table since the table has funcref type
                return [], [Instr('drop', ())]

        assert len(self.table_params) <= MAX_TABLE_PARAMS - 1
        table_loc_choice = len(self.table_params)
        table_loc = table_loc_choice + TABLE_PARAMS_START
        if table_loc_choice == len(self.table_params):
            self.table_params.append(TableParam(ty, table_loc, is_load))
        
        instr_seq = []
        instr_seq.append(Instr('i32.const', (table_loc,)))
        if is_load:
            instr_seq.append(Instr(f'table.get', (0,)))
            return instr_seq, []
        else:
            pre_instr_seq = instr_seq
            post_instr_seq = []
            post_instr_seq.append(Instr(f'table.set', (0,)))
            return pre_instr_seq, post_instr_seq

    def alloc_type_section(self, params, rets):
        assert type(params) == list and type(rets) == list
        assert not (len(params) == 0 and len(rets) == 0)
        assert not (len(params) == 0 and len(rets) == 1)

        try:
            return self.types.index((params, rets))
        except ValueError:
            self.types.append((params, rets))
            return len(self.types) - 1


class StackGenerator(Generator):
    def __init__(self):
        super().__init__()
        self.rule_provider = StackExtRuleProvider()
        self.template = ""
        with open(os.path.join(source_dir, "template.js"), "rt") as f:
            self.template = f.read()

    def gen_module(self, ctx: StackGenContext):
        ctx.generate(self.rule_provider)
        assert len(ctx.funcs) > 0

        sections = []

        # type sections
        type_sigs = [components.FunctionSig(params, rets) for params, rets in ctx.types]
        sections.append(components.TypeSection(*type_sigs))

        # main function
        main_func = ctx.funcs[0]
        main_func_component = components.Function('main', main_func.args, main_func.rets, main_func.locals, main_func.get_compat_instrs(), True)
        sections.append(main_func_component)

        # other functions
        for func_idx, func in enumerate(ctx.funcs[1:]):
            func_comp = components.Function(f'func{func_idx}', func.args, func.rets, func.locals, func.get_compat_instrs(), False)
            sections.append(func_comp)

        # table section
        sections.append(components.TableSection(components.Table('funcref', config.codegen_table_size, None)))

        # memory section
        sections.append(components.MemorySection(config.codegen_memory_max))

        # global section
        global_list = []
        for global_ty, global_node in ctx.globals:
            global_compat_instrs = [(g.name, g.operands[0]) for g in global_node.instrs]
            global_list.append(components.Global(global_ty, True, tuple(global_compat_instrs)))
        sections.append(components.GlobalSection(*global_list))

        exports = [components.Export(f'global{idx}', 'global', idx) for idx in range(len(global_list))] + \
                  [components.Export('mem', 'memory', 0), components.Export('table', 'table', 0)]

        sections.append(components.ExportSection(*exports))

        module = components.Module(*sections)
        return module

    def wrap_with_template(self, ctx, module):
        codebuf = "const code = new Uint8Array(["
        codebuf += ",".join([str(i) for i in module.to_bytes()])
        codebuf += "]);"

        args = []
        for itype in ctx.funcs[0].args:
            if itype in ('i32', 'f32', 'f64'):
                args.append('1')
            elif itype in ('i64',):
                args.append('1n')
            elif itype in ('v128', 'funcref', 'externref'):
                args.append('null') # TODO: change this
            else:
                assert False # unreachable
        
        arg_str = ','.join(args)

        code = self.template
        code = code.replace("WASM_EXECUTE_HOLDER", "") # not used for now
        code = code.replace("WASM_CODE_HOLDER", codebuf)
        code = code.replace("WASM_MEMORY_MAX", str(config.codegen_memory_max))
        return code

    @override
    def gen_code_info(self, seed, instrs_overwrite=[]):
        if config.codegen_is_random:
            rng = RandomRng(seed)
        else:
            rng = ConsumeRng(seed)
        
        ctx = StackGenContext(rng)
        module = self.gen_module(ctx)
        code = self.wrap_with_template(ctx, module)
        itypes = ctx.args
        otypes = ctx.rets
        return code, itypes, otypes, ctx.instrs

    @override
    def gen_wasm_info(self, seed, instrs_overwrite=[]):
        if config.codegen_is_random:
            rng = RandomRng(seed)
        else:
            rng = ConsumeRng(seed)
        
        ctx = StackGenContext(rng)
        module = self.gen_module(ctx)
        code = module.to_bytes()
        itypes = ctx.args
        otypes = ctx.rets
        return code, itypes, otypes, ctx.instrs
