import random
import subprocess
from overrides import override
import os
import sys
import re
import math
from collections import namedtuple

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(source_dir)
sys.path[0] = root_dir

import config
from codegen.instructions import INSTRUCTIONS, ALIGN_CANDIDATES
from codegen import components

Instr = namedtuple("Instr", ('name', 'operands'))

MEM_PAGE_SIZE = 2**16
MEM_PARAMS_START = 0
MAX_MEM_PARAMS = config.codegen_memory_max * MEM_PAGE_SIZE // 16

TABLE_PARAMS_START = 0
MAX_TABLE_PARAMS = config.codegen_table_size - TABLE_PARAMS_START

MemParam = namedtuple("MemParam", ('type', 'index', 'is_load'))
TableParam = namedtuple("TableParam", ('type', 'index', 'is_load'))

# noout used as a special meaning that the instruction doesn't output anything
TYPES = ['noout', 'i32', 'i64', 'f32', 'f64', 'v128', 'funcref', 'externref'] # externref not actually used?
ALLOWED_TYPES = [ty for ty in TYPES if not ty in config.codegen_blacklist_types]

BLOCKED_OPCODES = (
    'data.drop',
    'elem.drop', # unused

    'table.init',
    'memory.init', # also unused

    'table.grow',
    'memory.grow' # non-deterministic
)

MEMORY_OPCODES = (
    'i32.load',
    'i64.load',
    'f32.load',
    'f64.load',
    'i32.load8_s',
    'i32.load8_u',
    'i32.load16_s',
    'i32.load16_u',
    'i64.load8_s',
    'i64.load8_u',
    'i64.load16_s',
    'i64.load16_u',
    'i64.load32_s',
    'i64.load32_u',
    
    'i32.store',
    'i64.store',
    'f32.store',
    'f64.store',
    'i32.store8',
    'i32.store16',
    'i64.store8',
    'i64.store16',
    'i64.store32',

    # 'memory.init',
    'memory.size',
    # 'memory.grow', # non-deterministic
    'memory.copy',
    'memory.fill',
    # 'data.drop',

    'v128.load',
    'v128.load8x8_s',
    'v128.load8x8_u',
    'v128.load16x4_s',
    'v128.load16x4_u',
    'v128.load32x2_s',
    'v128.load32x2_u',
    'v128.load8_splat',
    'v128.load16_splat',
    'v128.load32_splat',
    'v128.load64_splat',
    'v128.store',

    'v128.load8_lane',
    'v128.load16_lane',
    'v128.load32_lane',
    'v128.load64_lane',
    'v128.store8_lane',
    'v128.store16_lane',
    'v128.store32_lane',
    'v128.store64_lane',

    'v128.load32_zero',
    'v128.load64_zero',
)

CONTROL_OPCODES = (
    'unreachable',
    'nop',
    'block',
    'loop',
    'if',
    'else',
    'end',
    'br',
    'br_if',
    'br_table',
    'return',
    'call',
    'call_indirect'
)

TABLE_OPCODES = ( # not exhaustive
    'table.get',
    'table.set',
    'table.copy',
    'table.size',
    'table.fill', 
)

FLOAT_CANON_OPCODES = (
    'f32.add',
    'f32.sub',
    'f32.mul',
    'f32.div',
    'f32.min',
    'f32.max',
    'f32.neg',
    'f32.sqrt',
    'f32.ceil',
    'f32.floor',
    'f32.trunc',
    'f32.nearest',
    'f64.add',
    'f64.sub',
    'f64.mul',
    'f64.div',
    'f64.min',
    'f64.max',
    'f64.neg',
    'f64.sqrt',
    'f64.ceil',
    'f64.floor',
    'f64.trunc',
    'f64.nearest',
    'f32x4.add',
    'f32x4.sub',
    'f32x4.mul',
    'f32x4.div',
    'f32x4.min',
    'f32x4.max',
    'f32x4.neg',
    'f32x4.sqrt',
    'f32x4.ceil',
    'f32x4.floor',
    'f32x4.trunc',
    'f32x4.nearest',
    'f64x2.add',
    'f64x2.sub',
    'f64x2.mul',
    'f64x2.div',
    'f64x2.min',
    'f64x2.max',
    'f64x2.neg',
    'f64x2.sqrt',
    'f64x2.ceil',
    'f64x2.floor',
    'f64x2.trunc',
    'f64x2.nearest',
    'f32.demote_f64',
    'f64.promote_f32',
    'f32x4.demote_f64x2_zero',
    'f64x2.promote_low_f32x4',
)

class InstrNode():
    def __init__(self, param_types, ret_types, instrs):
        assert all(map(lambda x: x in ALLOWED_TYPES+['anystack'], param_types))
        assert all(map(lambda x: x in ALLOWED_TYPES+['anystack'], ret_types))
        self.param_types = param_types
        self.ret_types = ret_types
        self.instrs = instrs # list of Instr
        self.conds = {} # dict of rule conditions
    
    @classmethod
    def from_single_instr(cls, instr: Instr): # from single instr
        inst = INSTRUCTIONS[instr.name]
        param_types = inst.input_types
        ret_types = inst.output_types
        instrs = [instr]
        return cls(param_types, ret_types, instrs)

    def __str__(self):
        return str((self.param_types, self.ret_types, self.instrs))


# Rule Provider
class RuleProvider():
    def __init__(self):
        pass
    
    def get_rule(self, ty, rng):
        pass

class ExtRuleProvider(RuleProvider):
    def __init__(self):
        super().__init__()

        self.typing_rules = {}
        self.extracted_rules = {}

        # extracted rules
        extractor_cwd = os.path.join(os.path.dirname(root_dir), "extractor")
        extractor_path = os.path.join(extractor_cwd, "target", "release", "extractor")
        args = [extractor_path]
        args.append(config.codegen_extractor_option)
        subp = subprocess.Popen(
            args, cwd=extractor_cwd, shell=False,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        try:
            result, err = subp.communicate(timeout=config.extractor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b""
        subp.kill()
        raw_rules = map(lambda x: eval(x), result.split(b"\n")[:-1])

        for raw_rule in raw_rules:
            ExtRuleProvider.add_raw_rule(raw_rule, self.extracted_rules)
        
        # typing rules
        args = [extractor_path]
        args.append("typing")
        subp = subprocess.Popen(
            args, cwd=extractor_cwd, shell=False,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        subp.stdout.flush()
        try:
            result, err = subp.communicate(timeout=config.extractor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b""
        subp.kill()
        raw_rules = map(lambda x: eval(x), result.split(b"\n")[:-1])

        for raw_rule in raw_rules:
            ExtRuleProvider.add_raw_rule(raw_rule, self.typing_rules)

        # memory 
        for op in MEMORY_OPCODES:
            if op in BLOCKED_OPCODES:
                continue
            
            inst_data = INSTRUCTIONS[op]
            assert len(inst_data.output_types) <= 1
            
            param_types = inst_data.input_types
            if len(inst_data.output_types) == 0:
                if 'noout' in config.codegen_blacklist_types:
                    continue
                ret_types = ()
            else:
                ret_types = inst_data.output_types
            
            instrs = []
            for _ in param_types:
                instrs.append(Instr('arg', (len(instrs),))) # arguments
            instrs.append(Instr(op, tuple([('oparg', idx) for idx, _ in enumerate(inst_data.operands)])))
            
            ExtRuleProvider.add_rule(param_types, ret_types, instrs, self.typing_rules)

        # control instructions (none... done manually in generation)
    
    @override
    def get_rule(self, ty, rng):
        if ty in self.extracted_rules and not rng.get_choice_prob(config.codegen_prob_use_typing): # extracted rules
            opcode = rng.get_choice_arr(list(self.extracted_rules[ty].keys()))
            rule = rng.get_choice_arr(self.extracted_rules[ty][opcode])
        else: # typing rules
            assert ty in self.typing_rules
            opcode = rng.get_choice_arr(list(self.typing_rules[ty].keys()))
            rule = rng.get_choice_arr(self.typing_rules[ty][opcode])
        return rule
            
    
    def add_raw_rule(raw_rule, rule_dict):
        assert len(raw_rule[1]) <= 1 # for now
        param_types = raw_rule[0]
        if len(raw_rule[1]) == 0:
            ret_types = ()
        else:
            ret_types = raw_rule[1]
        
        # ignore 3, which is condition for args, and non-existent
        oparg_conds_raw = raw_rule[4] # oparg conditions
        conds = {}
        for oparg_idx, oparg_conds in oparg_conds_raw:
            conds[oparg_idx] = ExtRuleProvider.postprocess_conds(oparg_conds)
        
        ExtRuleProvider.add_rule(param_types, ret_types, raw_rule[2], rule_dict)
    
    def add_rule(param_types, ret_types, instrs, rule_dict):
        if all(map(lambda x: x in ALLOWED_TYPES, param_types)) and all(map(lambda x: x in ALLOWED_TYPES, ret_types)):
            if len(ret_types) > 0:
                node = InstrNode(param_types, ret_types, instrs)
                node.conds = {}
                ty = ret_types[0]
            else:
                node = InstrNode(param_types, ret_types, instrs)
                node.conds = {}
                ty = 'noout'
            
            last_opcode = instrs[-1].name
            if last_opcode == 'arg': # ignore this case: unnecessary
                pass
            elif not ty in rule_dict:
                rule_dict[ty] = {last_opcode:[node]}
            elif not last_opcode in rule_dict[ty]:
                rule_dict[ty][last_opcode] = [node]
            else:
                rule_dict[ty][last_opcode].append(node)
        else:
            pass # ignore

    def postprocess_conds(conds):
        def eval_condexpr(condexpr):
            if type(condexpr) != tuple:
                return condexpr
            
            assert len(condexpr) > 0
            name = condexpr[0]
            if name == 'u64_sub':
                assert len(condexpr) == 3
                operand1 = eval_condexpr(condexpr[1])
                operand2 = eval_condexpr(condexpr[2])
                if type(operand1) == int and type(operand2) == int:
                    return operand1 - operand2
                else:
                    return (name, operand1, operand2)
            else:
                return tuple([name] + [eval_condexpr(x) for x in condexpr[1:]])

        processed_conds = [eval_condexpr(cond) for cond in conds]
        return processed_conds


# Generation Context
class GenContext():
    def __init__(self, rng):
        self.rng = rng

        # canonicalization
        # self.canonicalize_nans = True
        self.canonicalize_nans = config.engine_name != "wasmtime"
    
    def gen_operand(self, opcode, operand, oparg_idx, oparg_store, conds):
        if type(operand) == bytes:
            return operand
        elif type(operand) == tuple: # vec() type
            return operand
        
        assert type(operand) == str
        new_operand = 0
        if operand == 'labelidx':
            new_operand = 0
        elif operand == 'laneidx':
            # shape.extract_lane, shape.replace_lane
            match_obj = re.search('[if][0-9]+x[0-9]+', opcode)
            if match_obj:
                shape = match_obj.group(0)
                assert re.match(f'{shape}\.(extract|replace)_lane(_[us])?', opcode) != None
                lanemax = int(shape[shape.find('x')+1:])
                new_operand = self.rng.get_choice(lanemax)
            else:
                # v128.loadN_lane, v128.storeN_lane
                match_obj = re.search('(load|store)[0-9]+', opcode)
                if match_obj:
                    N = match_obj.group(0).replace('load','').replace('store','')
                    assert opcode == f'v128.load{N}_lane' or opcode == f'v128.store{N}_lane'
                    lanemax = 128 // int(N)
                    new_operand = self.rng.get_choice(lanemax)
                else:
                    assert False
        elif operand == 'byte16':
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(self.filter_operand_with_conds(opcode, operand, oparg_idx, config.codegen_interesting_v128, conds))
            elif not oparg_idx in conds:
                new_operand = self.rng.get_int(128)
            else:
                new_operand = self.gen_operand_with_conds(opcode, operand, oparg_idx, oparg_store, conds)
        elif operand == 'laneidx16':
            assert opcode == 'i8x16.shuffle' # laneidx < 32, 16 times
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(list(config.codegen_interesting_laneidx16)+[int.from_bytes((i,)*16, 'big') for i in range(32)])
            elif not oparg_idx in conds:
                new_operand = int.from_bytes([self.rng.get_int(5) for _ in range(16)], 'big')
            else:
                new_operand = self.gen_operand_with_conds(opcode, operand, oparg_idx, oparg_store, conds)
        elif operand == 'memarg':
            if 'atomic' in opcode:
                # atomic instructions must always have maximum alignment
                match_obj = re.findall('[0-9]+', opcode)
                if len(match_obj) == 0:
                    assert opcode == "memory.atomic.notify"
                    align = 4 # memory.atomic.notify
                else:
                    align = int(match_obj[-1]) // 8
                assert align & (align-1) == 0 # align == 2^n
                align = int(math.log2(align))
                new_operand = (align, 0) # (align, offset)
            else:
                assert opcode in ALIGN_CANDIDATES
                align_candidates = ALIGN_CANDIDATES[opcode]
                align = self.rng.get_choice_arr(align_candidates)

                if self.rng.get_choice_prob(config.codegen_prob_memarg_inbounds):
                    offset = self.rng.get_choice(config.codegen_memory_max * MEM_PAGE_SIZE)
                else:
                    offset = self.rng.get_int(32)
                new_operand = (align, offset)
        elif operand == 'i32':
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(self.filter_operand_with_conds(opcode, operand, oparg_idx, config.codegen_interesting_i32, conds))
            elif not oparg_idx in conds:
                new_operand = self.rng.get_int(32)
            else:
                new_operand = self.gen_operand_with_conds(opcode, operand, oparg_idx, oparg_store, conds)
        elif operand == 'i64':
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(self.filter_operand_with_conds(opcode, operand, oparg_idx, config.codegen_interesting_i64, conds))
            elif not oparg_idx in conds:
                new_operand = self.rng.get_int(64)
            else:
                new_operand = self.gen_operand_with_conds(opcode, operand, oparg_idx, oparg_store, conds)
        elif operand == 'f32':
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(config.codegen_interesting_f32)
            else:
                new_operand = self.rng.get_float()
        elif operand == 'f64':
            if self.rng.get_choice_prob(config.codegen_prob_const_use_interesting):
                new_operand = self.rng.get_choice_arr(config.codegen_interesting_f64)
            else:
                new_operand = self.rng.get_float()
        elif operand == 'tableidx':
            new_operand = 0 # only one table defined
        else:
            assert False
        
        # assert not oparg_idx in oparg_store
        oparg_store[oparg_idx] = new_operand
        return new_operand

    def gen_operand_with_conds(self, opcode, operand, oparg_idx, oparg_store, conds):
        oparg_conds = conds[oparg_idx] if oparg_idx in conds else []
        assert operand in ('i32', 'i64', 'byte16', 'laneidx16')
        if operand == 'i32':
            bitwidth = 32
        elif operand == 'i64':
            bitwidth = 64
        else:
            bitwidth = 128

        if operand == 'laneidx16':
            byteorder = 'big'
        else:
            byteorder = 'little'

        range_max = (1 << bitwidth) - 1 # assumed unsigned
        range_min = 0
        for cond in oparg_conds:
            if type(cond) == int:
                return cond
            elif cond[0] == 'le' and type(cond[1]) == int:
                if range_max > cond[1]:
                    range_max = cond[1]                
            elif cond[0] == 'lt' and type(cond[1]) == int:
                if range_max > cond[1]:
                    range_max = cond[1] - 1            
            elif cond[0] == 'ge' and type(cond[1]) == int:
                if range_min < cond[1]:
                    range_min = cond[1]
            elif cond[0] == 'gt' and type(cond[1]) == int:
                if range_min < cond[1]:
                    range_min = cond[1] - 1
            elif cond[0] == '_nonzero' or cond[0] == 'nonzero':
                if range_min == 0:
                    range_min = 1
            elif cond[0] == '_nonminusone':
                if range_max == (1 << bitwidth) - 1:
                    range_max = (1 << bitwidth) - 2
            elif cond[0] == '_minusone':
                return (1 << bitwidth) - 1
            elif cond[0] == '_pow2':
                assert len(cond) == 2
                var_idx = int(cond[1][len('var'):])
                var_cond = conds[var_idx] if var_idx in conds else []
                new_conds = conds.copy()
                new_conds[var_idx] = var_cond + [('lt', bitwidth)]
                return 2 ** self.gen_operand_with_conds(opcode, operand, var_idx, oparg_store, new_conds) # mix with others
            elif cond[0] == '_shuffle_dup8_from_imm':
                assert len(cond) == 2
                var_idx = int(cond[1][len('var'):])
                var_cond = conds[var_idx] if var_idx in conds else []
                new_conds = conds.copy()
                new_conds[var_idx] = var_cond + [('lane8',)]
                new_operand = self.gen_operand_with_conds(opcode, operand, var_idx, oparg_store, new_conds) # mix with others
                return 0x01010101010101010101010101010101 * new_operand
            elif cond[0] == '_shuffle_dup16_from_imm':
                assert len(cond) == 2
                var_idx = int(cond[1][len('var'):])
                var_cond = conds[var_idx] if var_idx in conds else []
                new_conds = conds.copy()
                new_conds[var_idx] = var_cond + [('lane16',)]
                new_operand = self.gen_operand_with_conds(opcode, operand, var_idx, oparg_store, new_conds) # mix with others
                return 0x00010001000100010001000100010001 * new_operand
            elif cond[0] == '_shuffle_dup32_from_imm':
                assert len(cond) == 2
                var_idx = int(cond[1][len('var'):])
                var_cond = conds[var_idx] if var_idx in conds else []
                new_conds = conds.copy()
                new_conds[var_idx] = var_cond + [('lane32',)]
                new_operand = self.gen_operand_with_conds(opcode, operand, var_idx, oparg_store, new_conds) # mix with others
                return 0x00000001000000010000000100000001 * new_operand
            elif cond[0] == '_shuffle_dup64_from_imm':
                assert len(cond) == 2
                var_idx = int(cond[1][len('var'):])
                var_cond = conds[var_idx] if var_idx in conds else []
                new_conds = conds.copy()
                new_conds[var_idx] = var_cond + [('lane64',)]
                new_operand = self.gen_operand_with_conds(opcode, operand, var_idx, oparg_store, new_conds) # mix with others
                return 0x00000000000000010000000000000001 * new_operand
            elif cond[0] == 'lane8':
                return self.rng.get_int(5) # 0 ~ 0x1f
            elif cond[0] == 'lane16':
                lane_raw = self.rng.get_int(10) # 0 ~ 0x1f
                val = (lane_raw & 0b11111) | ((lane_raw >> 5) & 0b11111)
                return val
            elif cond[0] == 'lane32':
                lane_raw = self.rng.get_int(20) # 0 ~ 0x1f
                val = (lane_raw & 0b11111) | ((lane_raw >> 5) & 0b11111) | ((lane_raw >> 10) & 0b11111) | ((lane_raw >> 15) & 0b11111)
                return val
            elif cond[0] == 'lane64':
                lane_raw = self.rng.get_int(40) # 0 ~ 0x1f
                val = (lane_raw & 0b11111) | ((lane_raw >> 5) & 0b11111) | ((lane_raw >> 10) & 0b11111) | ((lane_raw >> 15) & 0b11111) | \
                      ((lane_raw >> 20) & 0b11111) | ((lane_raw >> 25) & 0b11111) | ((lane_raw >> 30) & 0b11111) | ((lane_raw >> 35) & 0b11111)
                return val
            elif cond[0] == '_pshufd_lhs_imm':
                bytes_arr = [0, 1, 2, 3]*4
                for i in range(4):
                    imm = self.rng.get_int(2) # 0 ~ 3
                    for idx in range(4*i, 4*i+4):
                        bytes_arr[idx] += imm * 4
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pshufd_rhs_imm':
                bytes_arr = [0, 1, 2, 3]*4
                for i in range(4):
                    imm = self.rng.get_int(2) + 4 # 4 ~ 7
                    for idx in range(4*i, 4*i+4):
                        bytes_arr[idx] += imm * 4
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_shufps_imm':
                bytes_arr = [0, 1, 2, 3]*4
                for i in range(4):
                    imm = self.rng.get_int(2) # 0 ~ 3
                    if i in (2, 3):
                        imm += 4
                    for idx in range(4*i, 4*i+4):
                        bytes_arr[idx] += imm * 4
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_shufps_rev_imm':
                bytes_arr = [0, 1, 2, 3]*4
                for i in range(4):
                    imm = self.rng.get_int(2) # 0 ~ 3
                    if i in (0, 1):
                        imm += 4
                    for idx in range(4*i, 4*i+4):
                        bytes_arr[idx] += imm * 4
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pshuflw_lhs_imm':
                bytes_arr = [0, 1]*8
                for i in range(8):
                    if i in (4, 5, 6, 7):
                        imm = i
                    else:
                        imm = self.rng.get_int(2) # 0 ~ 3
                    for idx in range(2*i, 2*i+2):
                        bytes_arr[idx] += imm * 2
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pshuflw_rhs_imm':
                bytes_arr = [0, 1]*8
                for i in range(8):
                    if i in (4, 5, 6, 7):
                        imm = i
                    else:
                        imm = self.rng.get_int(2) + 8 # 0 ~ 3
                    for idx in range(2*i, 2*i+2):
                        bytes_arr[idx] += imm * 2
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pshufhw_lhs_imm':
                bytes_arr = [0, 1]*8
                for i in range(8):
                    if i in (0, 1, 2, 3):
                        imm = i
                    else:
                        imm = self.rng.get_int(2) + 4 # 4 ~ 7
                    for idx in range(2*i, 2*i+2):
                        bytes_arr[idx] += imm * 2
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pshufhw_rhs_imm':
                bytes_arr = [0, 1]*8
                for i in range(8):
                    if i in (0, 1, 2, 3):
                        imm = i + 8
                    else:
                        imm = self.rng.get_int(2) + 12 # 12 ~ 15
                    for idx in range(2*i, 2*i+2):
                        bytes_arr[idx] += imm * 2
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_palignr_imm_from_immediate':
                bytes_arr = list(range(16))
                imm = self.rng.get_choice(17) # 0 ~ 16
                for i in range(16):
                    bytes_arr[i] += imm
                return int.from_bytes(bytes_arr, byteorder)
            elif cond[0] == '_pblendw_imm':
                bytes_arr = [0, 1]*8
                for i in range(8):
                    imm = self.rng.get_int(1) * 8 + i 
                    for idx in range(2*i, 2*i+2):
                        bytes_arr[idx] += imm * 2
                return int.from_bytes(bytes_arr, byteorder)
            # elif cond[0] == '_splat64':
            # elif cond[0] == '_nonzero_hipart':
            # elif cond[0] == '_nonzero_lopart':
            # elif cond[0] == '_fits_in_32':
            # elif cond[0] == '_vconst_all_ones_or_all_zeros':
            # elif cond[0] == '_nonnegative':
            # elif cond[0] == '_sse_interps_lane_imm':
            # elif cond[0] == '_inverted':
            # elif cond[0] == '_swapped':
            # elif cond[0] == '_negate':
            # elif cond[0] == '_minusone':
            # elif cond[0] == '_shuffle64_from_imm':
            # elif cond[0] == '_shuffle32_from_imm':
            # elif cond[0] == '_shuffle16_from_imm':
            # elif cond[0] == '_u64_low32_bits_unset':
            # elif cond[0] == '_u128_replicated_u64':
            # elif cond[0] == '_u64_replicated_u32':
            # elif cond[0] == '_u32_replicated_u16':
            # elif cond[0] == '_u16_replicated_u8':

        if range_max <= range_min: # failed
            return self.rng.get_int(bitwidth)
        else:
            return self.rng.get_choice(range_max - range_min) + range_min

    def filter_operand_with_conds(self, opcode, operand, oparg_idx, vals, conds):
        oparg_conds = conds[oparg_idx] if oparg_idx in conds else []
        assert operand in ('i32', 'i64', 'byte16', 'laneidx16')
        if len(vals) == 0:
            print(opcode)
        assert len(vals) > 0
        if operand == 'i32':
            bitwidth = 32
        elif operand == 'i64':
            bitwidth = 64
        else:
            bitwidth = 128

        filtered_vals = list(vals)
        for cond in oparg_conds:
            if type(cond) == int:
                filtered_vals = list(filter(lambda x: x == cond, filtered_vals))
            elif cond[0] == 'le' and type(cond[1]) == int:
                filtered_vals = list(filter(lambda x: x <= cond[1], filtered_vals))
            elif cond[0] == 'lt' and type(cond[1]) == int:
                filtered_vals = list(filter(lambda x: x < cond[1], filtered_vals))
            elif cond[0] == 'ge' and type(cond[1]) == int:
                filtered_vals = list(filter(lambda x: x >= cond[1], filtered_vals))
            elif cond[0] == 'gt' and type(cond[1]) == int:
                filtered_vals = list(filter(lambda x: x > cond[1], filtered_vals))
            elif cond[0] == '_nonzero' or cond[0] == 'nonzero':
                filtered_vals = list(filter(lambda x: x != 0, filtered_vals))
            elif cond[0] == '_nonminusone':
                filtered_vals = list(filter(lambda x: x != (1 << bitwidth) - 1 and x != -1, filtered_vals))
            elif cond[0] == '_minusone':
                filtered_vals = list(filter(lambda x: x == -1, filtered_vals))
            elif cond[0] == '_pow2':
                assert len(cond) == 2
                filtered_vals = list(filter(lambda x: x == 2 ** (x.bit_length()), filtered_vals))
        
        if len(filtered_vals) == 0: # failed filtering
            return vals
        else:
            return tuple(filtered_vals)

    def perturb_operand(self, opcode, operand, operand_val):
        # doesn't contain max limit
        def perturb_int(val, min_limit, max_limit):
            assert type(val) == int
            assert max_limit > min_limit and val >= min_limit and max_limit > val
            if self.rng.get_choice_prob(config.codegen_prob_perturb):
                return val
            elif self.rng.get_choice_prob(1/3):
                if self.rng.get_choice_prob(1/3):
                    return max(val + 1, max_limit - 1)
                elif self.rng.get_choice_prob(1/2):
                    return min(val - 1, min_limit)
                else:
                    return max(min(-val, min_limit), max_limit - 1)
            elif self.rng.get_choice_prob(1/2):
                self.rng.get_choice_arr(list(filter(lambda x: x >= min_limit and x < max_limit, config.codegen_interesting_i64)))
            else: # re-make value
                self.rng.get_choice(max_limit - min_limit) + min_limit
        
        def perturb_float(val):
            assert type(val) == float
            if self.rng.get_choice_prob(config.codegen_prob_perturb):
                return val
            elif self.rng.get_choice_prob(1/3):
                if self.rng.get_choice_prob(1/3):
                    return val + 1.0
                elif self.rng.get_choice_prob(1/2):
                    return val - 1.0
                else:
                    return -val
            elif self.rng.get_choice_prob(1/2):
                self.rng.get_choice_arr(config.codegen_interesting_f64)
            else:
                return self.rng.get_float()

        if type(operand) == bytes:
            return operand
        elif type(operand) == tuple: # vec() type
            return operand
        
        assert type(operand) == str
        if operand == 'labelidx':
            return 0

        elif operand == 'laneidx':
            # shape.extract_lane, shape.replace_lane
            match_obj = re.search('[if][0-9]+x[0-9]+', opcode)
            if match_obj:
                shape = match_obj.group(0)
                assert re.match(f'{shape}\.(extract|replace)_lane(_[us])?', opcode) != None
                lanemax = int(shape[shape.find('x')+1:])
                return perturb_int(operand_val, 0, lanemax)
            
            # v128.loadN_lane, v128.storeN_lane
            match_obj = re.search('(load|store)[0-9]+', opcode)
            if match_obj:
                N = match_obj.group(0).replace('load','').replace('store','')
                assert opcode == f'v128.load{N}_lane' or opcode == f'v128.store{N}_lane'
                lanemax = 128 // int(N)
                return perturb_int(operand_val, 0, lanemax)
            
            assert False # unreachable
        elif operand == 'byte16':
            assert type(operand_val) == int
            bytes_arr = operand_val.to_bytes(16, 'little')
            new_operand = []
            for v in bytes_arr:
                new_operand.append(perturb_int(v, 0, 0x100))
            return int.from_bytes(new_operand, 'little')
        elif operand == 'laneidx16':
            assert opcode == 'i8x16.shuffle' # laneidx < 32, 16 times
            assert type(operand_val) == int
            laneidx_arr = operand_val.to_bytes(16, 'big')
            new_operand = []
            for v in laneidx_arr:
                new_operand.append(perturb_int(v, 0, 0x20))
            return int.from_bytes(new_operand, 'big')
        elif operand == 'memarg':
            if 'atomic' in opcode:
                # atomic instructions must always have maximum alignment
                match_obj = re.findall('[0-9]+', opcode)
                if len(match_obj) == 0:
                    assert opcode == "memory.atomic.notify"
                    align = 4 # memory.atomic.notify
                else:
                    align = int(match_obj[-1]) // 8
                assert align & (align-1) == 0 # align == 2^n
                align = int(math.log2(align))
                return (align, 0) # (align, offset)
            
            return (0, 0) # (align, offset)
        elif operand == 'i32':
            return perturb_int(operand_val, -(1 << 31), (1 << 31))
        elif operand == 'i64':
            return perturb_int(operand_val, -(1 << 63), (1 << 63))
        elif operand == 'f32':
            return perturb_float(operand_val)
        elif operand == 'f64':
            return perturb_float(operand_val)
        else:
            assert False


# Generator
class Generator():
    def __init__(self):
        pass

    def gen_code_info(self, seed, instrs_overwrite=[]):
        pass # should return code, itypes, otypes, instrs

    def gen_wasm_info(self, seed, instrs_overwrite=[]):
        pass # should return code, itypes, otypes, instrs


# Wrappers
class Wrapper():
    def __init__(self, generator):
        self.generator = generator

    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        return b""


class JSWrapper(Wrapper):
    def __init__(self, generator):
        super().__init__(generator)

    # WARNING: if compiler is not forced, optimization might occur due to repetitive execution
    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        code, itypes, otypes, instr_ast = self.generator.gen_code_info(seed, instrs_overwrite)
        rand = random.Random(seed)

        code += "try{instance.exports.main("
        init_args = []
        for itype in itypes:
            if itype == 'i32':
                init_args.append("0")
            elif itype == 'i64':
                init_args.append("0n")
            elif itype == 'f32' or itype == 'f64':
                init_args.append("0.0")
            elif itype == 'v128':
                init_args.append("0n")
                init_args.append("0n")
            else:
                pass
        code += ",".join(init_args)
        code += ")}catch(e){print(e)};\n"

        code += "const nan = NaN;\n"
        code += "const inf = Infinity;\n"
        code += "let mem = new BigUint64Array(memory.buffer);\n"

        i32_choice = config.run_interesting_i32+(rand.randint(-(1 << 31), (1 << 31)),)
        i64_choice = config.run_interesting_i64+(rand.randint(-(1 << 63), (1 << 63)),)
        f32_choice = config.run_interesting_f32+(rand.random(),)
        f64_choice = config.run_interesting_f64+(rand.random(),)
        v128_choice = [[val & ((1 << 64) - 1), val >> 64] for val in config.codegen_interesting_v128]
        v128_choice.append([rand.randint(0, (1 << 64) - 1) for _ in range(2)])

        code += f"let run_interesting_i32 = [{','.join([str(i) for i in i32_choice])}];\n"
        code += f"let run_interesting_i64 = [{','.join([str(i)+'n' for i in i64_choice])}];\n"
        code += f"let run_interesting_f32 = [{','.join([str(i) for i in f32_choice])}];\n"
        code += f"let run_interesting_f64 = [{','.join([str(i) for i in f64_choice])}];\n"
        code += f"let run_interesting_v128 = [{','.join(['['+','.join([str(j)+'n' for j in i])+']' for i in v128_choice])}];\n"

        if len(itypes) <= 2:
            # generate loop code
            cur_indent = 0
            args = []
            mem_args = []
            for itype in itypes:
                arg_name = f'a{len(args)+len(mem_args)}'
                code += ' '*4*cur_indent
                if type(itype) == MemParam:
                    code += f"for (let {arg_name} of run_interesting_{itype.type}) {{\n"
                    mem_args.append(arg_name)
                else:
                    code += f"for (let {arg_name} of run_interesting_{itype}) {{\n"
                    if itype == 'v128':
                        args.append(f'{arg_name}[0]')
                        args.append(f'{arg_name}[1]')
                    else:
                        args.append(arg_name)
                cur_indent += 1
            
            for idx, mem_arg in enumerate(mem_args):
                for off in range(2):
                    code += ' '*4*cur_indent
                    code += f"mem[{idx*2+off}]={mem_arg}[{off}];\n"

            args_str = ",".join(args)
            code += ' '*4*cur_indent
            code += f"try{{print({args_str}{',' if len(args_str) > 0 else ''}instance.exports.main({args_str}))}}catch(e){{print(e)}}\n"
            cur_indent -= 1
            
            while cur_indent >= 0:
                code += ' '*4*cur_indent
                code += "}\n"
                cur_indent -= 1
        else:
            # No proper way to seed Math.random()
            for _ in range(1000):
                args = []
                mem_args = []
                for itype in itypes:
                    if type(itype) == MemParam and itype.is_load:
                        assert itype.type == 'v128'
                        mem_args.append(rand.choice(v128_choice))
                    elif itype == 'i32':
                        args.append(str(rand.choice(i32_choice)))
                    elif itype == 'i64':
                        args.append(str(rand.choice(i64_choice))+'n')
                    elif itype == 'f32':
                        args.append(str(rand.choice(f32_choice)))
                    elif itype == 'f64':
                        args.append(str(rand.choice(f64_choice)))
                    elif itype == 'v128':
                        vals = rand.choice(v128_choice)
                        args.append(str(vals[0])+'n')
                        args.append(str(vals[1])+'n')
                
                for idx, mem_arg in enumerate(mem_args):
                    code += f"mem[{idx*2}]={hex(mem_arg[0])}n; mem[{idx*2+1}]={hex(mem_arg[1])}n;\n"
                
                args_str = ",".join(args)
                code += f"try{{print({args_str}{',' if len(args_str) > 0 else ''}instance.exports.main({args_str}))}}catch(e){{print(e)}}\n"
        
        code += "print(xxHash32(new Uint8Array(memory.buffer)));\n"

        return code.encode(), itypes, otypes, instr_ast


class RawWrapper(Wrapper):
    def __init__(self, generator):
        super().__init__(generator)

    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        return self.generator.gen_wasm_info(seed, instrs_overwrite)


def pretty_print_instrs(instrs):
    for instr in instrs:
        print(instr.name, instr.operands)

# in a simplified tuple list form
def describe_instrs(instrs):
    res = []
    for instr in instrs:
        res.append((instr.name, instr.operands))
    return res


# Evaluation
class XsmithWrapper(Wrapper):
    def __init__(self):
        super().__init__(None)

    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        xsmith_rkt_path = os.path.join(config.xsmith_codegen_path, "wasmlike.rkt")

        # generate
        gen_subp = subprocess.Popen(
            ["/usr/bin/racket", xsmith_rkt_path, "--seed", str(seed)], cwd=config.xsmith_codegen_path,
            shell=False, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        gen_subp.stdout.flush()
        try:
            gen_wat, _ = gen_subp.communicate(timeout=config.extractor_timeout_sec)
        except subprocess.TimeoutExpired:
            gen_wat = b""
        gen_subp.kill()

        # convert
        conv_subp = subprocess.Popen(
            ["/usr/bin/wat2wasm", "-", "--output=/dev/stdout"], cwd=config.xsmith_codegen_path,
            shell=False, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE
        )
        conv_subp.stdout.flush()
        try:
            result, _ = conv_subp.communicate(input=gen_wat, timeout=config.extractor_timeout_sec)
        except subprocess.TimeoutExpired:
            result = b""
        conv_subp.kill()

        return result, [], ['i32'], []


class XsmithJSWrapper(XsmithWrapper):
    def __init__(self):
        super().__init__()
        self.template = ""
        with open(os.path.join(source_dir, "xsmith-template.js"), "rt") as f:
            self.template = f.read()
        
    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        code, itypes, otypes, instr_ast = super().get_run_code(seed, workdir, instrs_overwrite)
        js_code = "const code = new Uint8Array(["
        js_code += ",".join([str(i) for i in code])
        js_code += "]);"

        return self.template.replace("WASM_CODE_HOLDER", js_code).encode(), itypes, otypes, instr_ast


class WasmSmithWrapper(Wrapper):
    def __init__(self):
        super().__init__(None)

    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        rand = random.Random(seed)

        # generate
        while True:
            args = [
                "wasm-tools", "smith", 
                "--min-types", "1", 
                "--min-funcs", "1", 
                "--min-memories", "1", 
                "--max-modules", "1", 
                "--simd-enabled", "true", 
                "--export-everything", "true",
                "--max-imports", "0",
                "--max-data-segments", "0",
                "--max-element-segments", "0",
                "--allow-start-export", "false",
                "--canonicalize-nans", "true" if config.wasm_smith_canon_nans else "false",
                "-t"
            ]
            subp = subprocess.Popen(
                args, shell=False,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
            )
            wat_code, err = subp.communicate(input=rand.randbytes(config.wasm_smith_seed_len))
            subp.kill()

            # ensure at least one function is generated
            if b"(func " in wat_code:
                break
        
        # change memory size, memory name, func name
        wat_code_split = []
        for x in wat_code.split(b'\n'):
            if x.startswith(b'  (memory '):
                wat_code_split.append(b'  (memory (;0;) 1 1)')
            elif x.endswith(b' (func 0))') and x.startswith(b'  (export '):
                wat_code_split.append(b'  (export "main" (func 0))')
            elif x.endswith(b' (memory 0))') and x.startswith(b'  (export '):
                wat_code_split.append(b'  (export "mem" (memory 0))')
            else:
                wat_code_split.append(x)
        wat_code = b'\n'.join(wat_code_split)
        
        subp = subprocess.Popen(
            ["wasm-tools", "parse", "-", "-o", "/dev/stdout"],
            shell=False,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        code, err = subp.communicate(input=wat_code)
        subp.kill()

        return code, [], [], []


class WasmSmithJSWrapper(Wrapper):
    def __init__(self):
        super().__init__(None)

        self.template = ""
        with open(os.path.join(source_dir, "template.js"), "rt") as f:
            self.template = f.read()

    @override
    def get_run_code(self, seed, workdir, instrs_overwrite=[]):
        rand = random.Random(seed)

        # generate types to avoid v128
        def choice_rand_types(rand: random.Random):
            ty_seq = []
            while True:
                choice = rand.choice(['i32', 'i64', 'f32', 'f64', 'stop'])
                if choice == 'stop':
                    break
                ty_seq.append(choice)
            return ty_seq
        
        param_types = choice_rand_types(rand)
        ret_types = choice_rand_types(rand)
        dummy_instrs = []
        for x in ret_types:
            if x in ('i32', 'i64'):
                dummy_instrs.append((f'{x}.const', 0))
            else:
                dummy_instrs.append((f'{x}.const', 0.0))
        ref_module = components.Module(
            components.TypeSection(components.FunctionSig(param_types, ret_types)),
            components.Function('main', param_types, ret_types, [], dummy_instrs, True)
        )
        with open(os.path.join(workdir, "ref.wasm"), "wb") as f:
            f.write(ref_module.to_bytes())

        # generate
        while True:
            args = [
                "wasm-tools", "smith", 
                "--min-types", "1", 
                "--min-funcs", "1", 
                "--min-memories", "1", 
                "--max-modules", "1", 
                "--simd-enabled", "true",
                "--max-imports", "0",
                "--max-data-segments", "0",
                "--max-element-segments", "0",
                "--allow-start-export", "false",
                "--canonicalize-nans", "true" if config.wasm_smith_canon_nans else "false",
                "--exports", os.path.join(workdir, "ref.wasm"),
                "-t"
            ]
            if config.engine_name == "sm":
                args.append("--disallow-traps")
                args.append("true")
            subp = subprocess.Popen(
                args, shell=False,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL
            )
            wat_code, err = subp.communicate(input=rand.randbytes(config.wasm_smith_seed_len))
            subp.kill()

            # ensure at least one function is generated
            if b"(func " in wat_code:
                break
        
        # change memory size, memory name, func name
        wat_code_split = []
        for x in wat_code.split(b'\n'):
            if x.startswith(b'  (memory '):
                wat_code_split.append(b'  (memory (;0;) 1 1)')
                wat_code_split.append(b'  (export "mem" (memory 0))')
            else:
                wat_code_split.append(x)
        wat_code = b'\n'.join(wat_code_split)
        
        subp = subprocess.Popen(
            ["wasm-tools", "parse", "-", "-o", "/dev/stdout"],
            shell=False,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE
        )
        wasm_code, err = subp.communicate(input=wat_code)
        subp.kill()

        # js code buffer
        codebuf = "const code = new Uint8Array(["
        codebuf += ",".join([str(i) for i in wasm_code])
        codebuf += "]);"

        args = []
        for itype in param_types:
            if itype in ('i32', 'f32', 'f64'):
                args.append('1')
            elif itype in ('i64',):
                args.append('1n')
            else:
                assert False # unreachable
        
        arg_str = ','.join(args)

        code = self.template
        code = code.replace("WASM_EXECUTE_HOLDER", "") # not used for now
        code = code.replace("WASM_CODE_HOLDER", codebuf)
        code = code.replace("WASM_MEMORY_MAX", str(config.codegen_memory_max))
        
        # js code wrapper (Copied from JSWrapper)
        code += "try{instance.exports.main("
        init_args = []
        for itype in param_types:
            if itype == 'i32':
                init_args.append("0")
            elif itype == 'i64':
                init_args.append("0n")
            elif itype == 'f32' or itype == 'f64':
                init_args.append("0.0")
            elif itype == 'v128':
                init_args.append("0n")
                init_args.append("0n")
            else:
                pass
        code += ",".join(init_args)
        code += ")}catch(e){print(e)};\n"

        code += "const nan = NaN;\n"
        code += "const inf = Infinity;\n"
        code += "let mem = new BigUint64Array(memory.buffer);\n"

        i32_choice = config.run_interesting_i32+(rand.randint(-(1 << 31), (1 << 31)),)
        i64_choice = config.run_interesting_i64+(rand.randint(-(1 << 63), (1 << 63)),)
        f32_choice = config.run_interesting_f32+(rand.random(),)
        f64_choice = config.run_interesting_f64+(rand.random(),)
        v128_choice = [[val & ((1 << 64) - 1), val >> 64] for val in config.codegen_interesting_v128]
        v128_choice.append([rand.randint(0, (1 << 64) - 1) for _ in range(2)])

        code += f"let run_interesting_i32 = [{','.join([str(i) for i in i32_choice])}];\n"
        code += f"let run_interesting_i64 = [{','.join([str(i)+'n' for i in i64_choice])}];\n"
        code += f"let run_interesting_f32 = [{','.join([str(i) for i in f32_choice])}];\n"
        code += f"let run_interesting_f64 = [{','.join([str(i) for i in f64_choice])}];\n"
        code += f"let run_interesting_v128 = [{','.join(['['+','.join([str(j)+'n' for j in i])+']' for i in v128_choice])}];\n"

        if len(param_types) <= 2:
            # generate loop code
            cur_indent = 0
            args = []
            mem_args = []
            for itype in param_types:
                arg_name = f'a{len(args)+len(mem_args)}'
                code += ' '*4*cur_indent
                if type(itype) == MemParam:
                    code += f"for (let {arg_name} of run_interesting_{itype.type}) {{\n"
                    mem_args.append(arg_name)
                else:
                    code += f"for (let {arg_name} of run_interesting_{itype}) {{\n"
                    if itype == 'v128':
                        args.append(f'{arg_name}[0]')
                        args.append(f'{arg_name}[1]')
                    else:
                        args.append(arg_name)
                cur_indent += 1
            
            for idx, mem_arg in enumerate(mem_args):
                for off in range(2):
                    code += ' '*4*cur_indent
                    code += f"mem[{idx*2+off}]={mem_arg}[{off}];\n"

            args_str = ",".join(args)
            code += ' '*4*cur_indent
            code += f"try{{print({args_str}{',' if len(args_str) > 0 else ''}instance.exports.main({args_str}))}}catch(e){{print(e)}}\n"
            cur_indent -= 1
            
            while cur_indent >= 0:
                code += ' '*4*cur_indent
                code += "}\n"
                cur_indent -= 1
        else:
            # No proper way to seed Math.random()
            for _ in range(1000):
                args = []
                mem_args = []
                for itype in param_types:
                    if type(itype) == MemParam and itype.is_load:
                        assert itype.type == 'v128'
                        mem_args.append(rand.choice(v128_choice))
                    elif itype == 'i32':
                        args.append(str(rand.choice(i32_choice)))
                    elif itype == 'i64':
                        args.append(str(rand.choice(i64_choice))+'n')
                    elif itype == 'f32':
                        args.append(str(rand.choice(f32_choice)))
                    elif itype == 'f64':
                        args.append(str(rand.choice(f64_choice)))
                    elif itype == 'v128':
                        vals = rand.choice(v128_choice)
                        args.append(str(vals[0])+'n')
                        args.append(str(vals[1])+'n')
                
                for idx, mem_arg in enumerate(mem_args):
                    code += f"mem[{idx*2}]={hex(mem_arg[0])}n; mem[{idx*2+1}]={hex(mem_arg[1])}n;\n"
                
                args_str = ",".join(args)
                code += f"try{{print({args_str}{',' if len(args_str) > 0 else ''}instance.exports.main({args_str}))}}catch(e){{print(e)}}\n"
        
        code += "print(xxHash32(new Uint8Array(memory.buffer)));\n"

        return code.encode(), param_types, ret_types, []
