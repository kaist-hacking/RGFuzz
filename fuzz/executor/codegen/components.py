"""
The field classes to represent a WASM program.
"""
import os
import sys

source_dir = os.path.dirname(os.path.abspath(__file__))
root_dir = os.path.dirname(source_dir)
sys.path.append(root_dir)

from io import BytesIO
from codegen.instructions import INSTRUCTIONS
from codegen import encode

LANG_TYPES = {
    'i32': b'\x7f',
    'i64': b'\x7e',
    'f32': b'\x7d',
    'f64': b'\x7c',
    'v128': b'\x7b',
    'funcref': b'\x70',
    'externref': b'\x6f',
    'func': b'\x60',
    'emptyblock': b'\x40',  # pseudo type for representing an empty block_type
}
# 'any' for specially used in Instruction

def lang_types_bit_len(ty):
    if ty in ('i32', 'f32'):
        return 32
    elif ty in ('i64', 'f64'):
        return 64
    elif ty == 'v128':
        return 128
    elif ty in ('funcref', 'externref'):
        return 4 # TODO: not sure
    else:
        assert False # should not happen

def is_float(ty):
    if ty in ('f32', 'f64'):
        return True
    else:
        return False

class WASMComponent:
    """Base class for representing components of a WASM module, from the module
    to sections and instructions. These components can be shown as text or
    written as bytes.
    
    Methods:
    * `to_bytes()` - Get the bytes that represent the binary WASM for this component.
    * `show()` - Print a textual representation of the component.
    * `to_file(f)` - Write the binary representation of this component to a file.
    * `to_text()` - Return a textual representation of this component.
    
    """
    
    __slots__ = []
    
    def __init__(self):
        pass
    
    def __repr__(self):
        return '<WASM-%s>' % self.__class__.__name__

    def _get_sub_text(self, subs, multiline=False):
        # Collect sub texts
        texts = []
        charcount = 0
        haslf = False
        for sub in subs:
            if isinstance(sub, WASMComponent):
                text = sub.to_text()
            else:
                text = repr(sub)
            charcount += len(text)
            texts.append(text)
            haslf = haslf or '\n' in text
        # Put on one line or multiple lines
        if multiline or haslf or charcount > 70:
            lines = []
            for text in texts:
                for line in text.splitlines():
                    lines.append(line)
            lines = ['    ' + line for line in lines]
            return '\n'.join(lines)
        else:
            return ', '.join(texts)
    
    def to_bytes(self):
        """ Get the bytes that represent the binary WASM for this component.
        """
        f = BytesIO()
        self.to_file(f)
        return f.getvalue()
    
    def show(self):
        """ Print a textual representation of the component.
        """
        print(self.to_text())
    
    def to_file(self, f):
        """ Write the binary representation of this component to a file.
        Implemented in the subclasses.
        """
        raise NotImplementedError()
    
    def to_text(self):
        """ Return a textual representation of this component.
        Implemented in the subclasses.
        """
        raise NotImplementedError()


class Module(WASMComponent):
    """ Class to represent a WASM module; the toplevel unit of code.
    The subcomponents of a module are objects that derive from `Section`.
    It is recommended to provide `Function` and `ImportedFunction` objects,
    from which the module will polulate the function-related sections, and
    handle the binding of the function index space.
    """
    
    __slots__ = ['sections', 'func_id_to_index']
    
    def __init__(self, *sections):
        # Process sections, filter out high-level functions
        self.sections = []
        has_lowlevel_funcs = False
        functions = []
        func_sigs = []
        start_section = None
        import_section = None
        export_section = None
        for section in sections:
            if isinstance(section, (Function, ImportedFunction)):
                functions.append(section)
            elif isinstance(section, TypeSection):
                func_sigs += list(section.functionsigs)
            elif isinstance(section, Section):
                self.sections.append(section)
                if isinstance(section, CodeSection):
                    has_lowlevel_funcs = True
                elif isinstance(section, StartSection):
                    start_section = section
                elif isinstance(section, ImportSection):
                    import_section = section
                elif isinstance(section, ExportSection):
                    export_section = section
            else:
                raise TypeError('Module expects a Function, ImportedFunction, or a Section.')
        
        # Process high level function desctiptions?
        if functions and has_lowlevel_funcs:
            raise TypeError('Module cannot have both Functions/ImportedFunctions and FunctionDefs/FunctionSigs.')
        elif functions:
            self._process_functions(func_sigs, functions, start_section, import_section, export_section)
        
        # Sort the sections
        self.sections.sort(key=lambda x: x.id)
        
        # Prepare functiondefs
        for section in reversed(self.sections):
            if isinstance(section, CodeSection):
                for funcdef in section.functiondefs:
                    funcdef.module = self
                break 
    
    def _process_functions(self, func_sigs, functions, start_section, import_section, export_section):
        
        # Prepare processing functions. In the order of imported and then defined,
        # because that's the order of the function index space. We use the function
        # index also for the signature index, though that's not strictly necessary
        # (e.g. functions can share a signature).
        # Function index space is used in, calls, exports, elementes, start function.
        auto_sigs = func_sigs.copy()
        auto_defs = []
        auto_imports = []
        auto_exports = []
        auto_start = None
        function_index = 0
        self.func_id_to_index = {}
        # Process imported functions
        for func in functions:
            if isinstance(func, ImportedFunction):
                auto_sigs.append(FunctionSig(func.params, func.returns))
                auto_imports.append(Import(func.modname, func.fieldname, 'function', function_index))
                if func.export:
                    auto_exports.append((func.idname, 'function', function_index))
                self.func_id_to_index[func.idname] = function_index
                function_index += 1
        # Process defined functions
        for func in functions:
            if isinstance(func, Function):
                auto_sigs.append(FunctionSig(func.params, func.returns))
                auto_defs.append(FunctionDef(func.locals, *func.instructions))
                if func.export:
                    auto_exports.append(Export(func.idname, 'function', function_index))
                if func.idname == '$main' and start_section is None:
                    auto_start = StartSection(function_index)
                self.func_id_to_index[func.idname] = function_index
                function_index += 1
        
        # Insert auto-generated function sigs and defs
        self.sections.append(TypeSection(*auto_sigs))
        self.sections.append(CodeSection(*auto_defs))
        # Insert auto-generated imports
        if import_section is None:
            import_section = ImportSection()
            self.sections.append(import_section)
        import_section.imports.extend(auto_imports)
        # Insert auto-generated exports
        if export_section is None:
            export_section = ExportSection()
            self.sections.append(export_section)
        export_section.exports.extend(auto_exports)
        # Insert function section
        self.sections.append(FunctionSection(*range(len(auto_imports)+len(func_sigs), function_index+len(func_sigs))))
        # Insert start section
        if auto_start is not None:
            self.sections.append(auto_start)
    
    def to_text(self):
        return 'Module(\n' + self._get_sub_text(self.sections, True) + '\n)'
    
    def to_file(self, f):
        f.write(b'\x00asm')
        f.write(encode.packu32(1))  # version, must be 1 for now
        for section in self.sections:
            section.to_file(f)


class Function:
    """ High-level description of a function. The linking is resolved
    by the module.
    """
    
    __slots__ = ['idname', 'params', 'returns', 'locals', 'instructions', 'export']
    
    def __init__(self, idname, params=None, returns=None, locals=None, instructions=None, export=False):
        assert isinstance(idname, str)
        assert isinstance(params, (tuple, list))
        assert isinstance(returns, (tuple, list))
        assert isinstance(locals, (tuple, list))
        assert isinstance(instructions, (tuple, list))
        self.idname = idname
        self.params = params
        self.returns = returns
        self.locals = locals
        self.instructions = instructions
        self.export = bool(export)


class ImportedFunction:
    """ High-level description of an imported function. The linking is resolved
    by the module.
    """
    
    __slots__ = ['idname', 'params', 'returns', 'modname', 'fieldname', 'export']
    
    def __init__(self, idname, params, returns, modname, fieldname, export=False):
        assert isinstance(idname, str)
        assert isinstance(params, (tuple, list))
        assert isinstance(returns, (tuple, list))
        assert isinstance(modname, str)
        assert isinstance(fieldname, str)
        self.idname = idname
        self.params = params
        self.returns = returns
        self.modname = modname
        self.fieldname = fieldname
        self.export = bool(export)


## Sections


class Section(WASMComponent):
    """Base class for module sections.
    """
    
    __slots__ = []
    id = -1
    
    def to_text(self):
        return '%s()' % self.__class__.__name__
    
    def to_file(self, f):
        f2 = BytesIO()
        self.get_binary_section(f2)
        payload = f2.getvalue()
        id = self.id
        assert id >= 0
        # Write it all
        f.write(encode.packvu7(id))
        f.write(encode.packvu32(len(payload)))
        if id == 0:  # custom section for debugging, future, or extension
            type = self.__cass__.__name__.lower().split('section')[0]
            f.write(encode.packstr(type))
        f.write(payload)
    
    def get_binary_section(self, f):
        raise NotImplementedError()  # Sections need to implement this


class TypeSection(Section):
    """ Defines signatures of functions that are either imported or defined in this module.
    """
    
    __slots__ = ['functionsigs']
    id = 1
    
    def __init__(self, *functionsigs):
        for i, functionsig in enumerate(functionsigs):
            assert isinstance(functionsig, FunctionSig)
            # TODO: remove this?
            functionsig.index = i  # so we can resolve the index in Import objects
        self.functionsigs = functionsigs
    
    def to_text(self):
        return 'TypeSection(\n' + self._get_sub_text(self.functionsigs, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.functionsigs)))  # count
        for functionsig in self.functionsigs:
            functionsig.to_file(f)


class ImportSection(Section):
    """ Defines the things to be imported in a module.
    """
    __slots__ = ['imports']
    id = 2
    
    def __init__(self, *imports):
        for imp in imports:
            assert isinstance(imp, Import)
        self.imports = list(imports)
    
    def to_text(self):
        return 'ImportSection(\n' + self._get_sub_text(self.imports, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.imports)))  # count
        for imp in self.imports:
            imp.to_file(f)


class FunctionSection(Section):
    """ Declares for each function defined in this module which signature is
    associated with it. The items in this sections match 1-on-1 with the items
    in the code section.
    """
    
    __slots__ = ['indices']
    id = 3
    
    def __init__(self, *indices):
        for i in indices:
            assert isinstance(i, int)
        self.indices = indices  # indices in the Type section
    
    def to_text(self):
        return 'FunctionSection(' + ', '.join([str(i) for i in self.indices]) + ')'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.indices)))
        for i in self.indices:
            f.write(encode.packvu32(i))


class TableSection(Section):
    """ Table Section
    """
    __slots__ = ['tables']
    id = 4

    def __init__(self, *tables):
        for table in tables:
            assert isinstance(table, Table)
        self.tables = list(tables)
    
    def to_text(self):
        return 'TableSection(\n' + self._get_sub_text(self.tables, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.tables)))
        for table in self.tables:
            table.to_file(f)


class MemorySection(Section):
    """ Declares initial (and max) sizes of linear memory, expressed in
    WASM pages (64KiB). Only one default memory can exist in the MVP.
    """
    __slots__ = ['entries']
    id = 5
    
    def __init__(self, *entries):
        assert len(entries) == 1   # in MVP
        self.entries = entries
        
    def to_text(self):
        return 'MemorySection(' + ', '.join([str(i) for i in self.entries]) + ')'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.entries)))
        for entrie in self.entries:
            # resizable_limits
            if isinstance(entrie, int):
                entrie = (entrie, )
            if len(entrie) == 1:
                f.write(encode.packvu1(0))
                f.write(encode.packvu32(entrie[0]))  # initial, no max
            else:
                f.write(encode.packvu1(1))
                f.write(encode.packvu32(entrie[0]))  # initial
                f.write(encode.packvu32(entrie[1]))  # maximum


class GlobalSection(Section):
    """ Defines the globals in a module. WIP.
    """
    __slots__ = ['globals']
    id = 6
    
    def __init__(self, *globals):
        for _global in globals:
            assert isinstance(_global, Global)
        self.globals = list(globals)
    
    def to_text(self):
        return 'GlobalSection(\n' + self._get_sub_text(self.globals, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.globals)))
        for _global in self.globals:
            _global.to_file(f)


class ExportSection(Section):
    """ Defines the names that this module exports.
    """
    __slots__ = ['exports']
    id = 7
    
    def __init__(self, *exports):
        for export in exports:
            assert isinstance(export, Export)
        self.exports = list(exports)
    
    def to_text(self):
        return 'ExportSection(\n' + self._get_sub_text(self.exports, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.exports)))
        for export in self.exports:
            export.to_file(f)
    

class StartSection(Section):
    """ Provide the index of the function to call at init-time. The func must
    have zero params and return values.
    """
    
    __slots__ = ['index']
    id = 8
    
    def __init__(self, index):
        self.index = index
    
    def to_text(self):
        return 'StartSection(' + str(self.index) + ')'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(self.index))


class ElementSection(Section):
    """ To initialize table elements. WIP.
    """
    __slots__ = []
    id = 9


class CodeSection(Section):
    """ The actual code for a module, one CodeSection per function.
    """
    __slots__ = ['functiondefs']
    id = 10
    
    def __init__(self, *functiondefs):
        for functiondef in functiondefs:
            assert isinstance(functiondef, FunctionDef)
        self.functiondefs = functiondefs
    
    def to_text(self):
        return 'CodeSection(\n' + self._get_sub_text(self.functiondefs, True) + '\n)'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.functiondefs)))
        for functiondef in self.functiondefs:
            functiondef.to_file(f)


class DataSection(Section):
    """ Initialize the linear memory.
    Note that the initial contents of linear memory are zero.
    """
    
    __slots__ = ['chunks']
    id = 11
    
    def __init__(self, *chunks):
        self.chunks = []
        for chunk in chunks:
            assert len(chunk) == 3  # index, offset, bytes
            assert chunk[0] == 0  # always 0 in MVP
            assert isinstance(chunk[2], bytes)
    
    def to_text(self):
        chunkinfo = [(chunk[0], chunk[1], len(chunk[2])) for chunk in self.chunks]
        return 'DataSection(' + ', '.join([str(i) for i in chunkinfo]) + ')'
    
    def get_binary_section(self, f):
        f.write(encode.packvu32(len(self.chunks)))
        for chunk in self.chunks:
            f.write(encode.packvu32(chunk[0]))
            #ff.write(encode.packvu32(chunk[1]))
            Instruction('i32.const', chunk[1]).to_file(f)  # TODO: change this
            f.write(encode.packvu32(len(chunk[2])))


## Non-section components


class Import(WASMComponent):
    """ Import objects (from other wasm modules or from the host environment).
    The type argument is an index in the type-section (signature) for funcs
    and a string type for table, memory and global.
    """
    
    __slots__ = ['modname', 'fieldname', 'kind', 'type']
    
    def __init__(self, modname, fieldname, kind, type):
        self.modname = modname
        self.fieldname = fieldname
        self.kind = kind
        self.type = type  # the signature-index for funcs, the type for table, memory or global
    
    def to_text(self):
        return 'Import(%r, %r, %r, %r)' % (self.modname, self.fieldname, self.kind, self.type)
    
    def to_file(self, f):
        f.write(encode.packstr(self.modname))
        f.write(encode.packstr(self.fieldname))
        if self.kind == 'function':
            f.write(b'\x00')
            f.write(encode.packvu32(self.type))
        elif self.kind == 'memory':
            f.write(b'\x02')
            if type(self.type) == tuple: # (min, max)
                assert len(self.type) == 2
                f.write(b'\x01')
                f.write(encode.packvu32(self.type[0]))
                f.write(encode.packvu32(self.type[1]))
            else:
                assert type(self.type) == int # (min, None)
                f.write(b'\x00')
                f.write(encode.packvu32(self.type))
        else:
            raise RuntimeError('Can only import functions for now')


class Export(WASMComponent):
    """ Export an object defined in this module. The index is the index
    in the corresponding index space (e.g. for functions this is the
    function index space which is basically the concatenation of
    functions in the import and code sections).
    """
    
    __slots__ = ['name', 'kind', 'index']
    
    def __init__(self, name, kind, index):
        self.name = name
        self.kind = kind
        self.index = index
    
    def to_text(self):
        return 'Export(%r, %r, %i)' % (self.name, self.kind, self.index)
    
    def to_file(self, f):
        f.write(encode.packstr(self.name))
        if self.kind == 'function':
            f.write(b'\x00')
            f.write(encode.packvu32(self.index))
        elif self.kind == 'table':
            f.write(b'\x01')
            f.write(encode.packvu32(self.index))
        elif self.kind == 'memory':
            f.write(b'\x02')
            f.write(encode.packvu32(self.index))
        elif self.kind == 'global':
            f.write(b'\x03')
            f.write(encode.packvu32(self.index))
        else:
            raise RuntimeError('Invalid kind')


class Global(WASMComponent):
    """ Defines globals 
    """

    __slots__ = ['valtype', 'mut', 'instructions', 'module']

    def __init__(self, valtype, mut, instructions):
        assert type(valtype) == str
        assert type(mut) == bool
        self.valtype = valtype
        self.mut = mut
        self.instructions = []
        for instruction in instructions:
            if isinstance(instruction, str):
                instruction = Instruction(instruction) # TODO: change this
            elif isinstance(instruction, tuple):
                instruction = Instruction(*instruction) # TODO: change this
            assert isinstance(instruction, Instruction)
            self.instructions.append(instruction)
        self.module = None
    
    def to_text(self):
        s = 'Global((' + 'mut' if self.mut else 'const'
        s += ' ' + self.valtype + ') '
        s += self._get_sub_text(self.instructions, False)
        s += ')'
        return s

    def to_file(self, f):
        f.write(LANG_TYPES[self.valtype]) # globaltype
        if self.mut:
            f.write(b'\x01')
        else:
            f.write(b'\x00')
        for instruction in self.instructions: # expr (init)
            instruction.to_file(f, self.module)
        f.write(b'\x0b')


class Table(WASMComponent):
    """ Defines tables
    """

    __slots__ = ['tabletype', 'min', 'max']

    def __init__(self, tabletype, min, max=None):
        assert tabletype in ('funcref', 'externref')
        assert type(min) == int
        assert type(max) == int or max == None

        self.tabletype = tabletype
        self.min = min
        self.max = max
    
    def to_text(self):
        s = 'Table(' + str(min) + ' '
        s += str(max)+' ' if max != None else ''
        s += self.tabletype + ')'
        return s

    def to_file(self, f):
        f.write(LANG_TYPES[self.tabletype]) # tabletype
        if self.max == None:
            f.write(b'\x00')
            f.write(encode.packvu32(self.min))
        else:
            f.write(b'\x01')
            f.write(encode.packvu32(self.min))
            f.write(encode.packvu32(self.max))


class FunctionSig(WASMComponent):
    """ Defines the signature of a WASM module that is imported or defined in
    this module.
    """
    __slots__ = ['params', 'returns', 'index']
    
    def __init__(self, params=(), returns=()):
        self.params = params
        self.returns = returns
        self.index = None
    
    def to_text(self):
        return 'FunctionSig(%r, %r)' % (list(self.params), list(self.returns))
    
    def to_file(self, f):
        f.write(b'\x60')  # form -> nonfunctions may also be supported in the future
        f.write(encode.packvu32(len(self.params)))  # params
        for paramtype in self.params:
            f.write(LANG_TYPES[paramtype])
        f.write(encode.packvu1(len(self.returns)))  # returns
        for rettype in self.returns:
            f.write(LANG_TYPES[rettype])


class FunctionDef(WASMComponent):
    """ The definition (of the body) of a function. The instructions can be
    Instruction instances or strings/tuples describing the instruction.
    """
    
    __slots__ = ['locals', 'instructions', 'module']
    
    def __init__(self, locals, *instructions):
        for loc in locals:
            assert isinstance(loc, str)  # valuetype
        self.locals = locals
        self.instructions = []
        for instruction in instructions:
            if isinstance(instruction, str):
                instruction = Instruction(instruction) # TODO: change this
            elif isinstance(instruction, tuple):
                instruction = Instruction(*instruction) # TODO: change this
            assert isinstance(instruction, Instruction)
            self.instructions.append(instruction)
        self.module = None
    
    def to_text(self):
        s = 'FunctionDef(' + str(list(self.locals)) + '\n'
        s += self._get_sub_text(self.instructions, True)
        s += '\n)'
        return s
    
    def to_file(self, f):
        
        # Collect locals by type
        local_entries = []  # list of (count, type) tuples
        for loc_type in self.locals:
            if local_entries and local_entries[-1] == loc_type:
                local_entries[-1] = local_entries[-1][0] + 1, loc_type
            else:
                local_entries.append((1, loc_type))
        
        f3 = BytesIO()
        f3.write(encode.packvu32(len(local_entries)))  # number of local-entries in this func
        for localentry in local_entries:
            f3.write(encode.packvu32(localentry[0]))  # number of locals of this type
            f3.write(LANG_TYPES[localentry[1]])
        for instruction in self.instructions:
            instruction.to_file(f3, self.module)
        f3.write(b'\x0b')  # end
        body = f3.getvalue()
        f.write(encode.packvu32(len(body)))  # number of bytes in body
        f.write(body)


class Instruction(WASMComponent):
    """ Class to represent an instruction. Can have nested instructions, which
    really just come after it (so it only allows semantic sugar for blocks and loops.
    """
    
    def __init__(self, type, *args):
        self.type = type.lower()
        # assert self.type in INSTRUCTIONS.keys() # check is done later
        self.args = args
    
    def __repr__(self):
        return '<Instruction %s>' % self.name
    
    def to_text(self):
        subtext = self._get_sub_text(self.args)
        if '\n' in subtext:
            return 'Instruction(' + repr(self.name) + ',\n' + subtext + '\n)'
        else:
            return 'Instruction(' + repr(self.name) + ', ' + subtext + ')'
    
    def to_file(self, f, m):
        if self.type not in INSTRUCTIONS:
            raise TypeError('Unknown instruction %r' % self.type)
        
        opcode, operands, _, _ = INSTRUCTIONS[self.type]
        f.write(opcode)

        assert len(self.args) == len(operands)
        for op, arg in zip(operands, self.args):
            if type(op) == bytes:
                f.write(op)
            elif op.endswith("idx"):
                assert isinstance(arg, int)
                f.write(encode.packvu32(arg))
            elif op.startswith("vec("):
                assert isinstance(arg, (tuple, list))
                op_inner = op[4:-1]
                f.write(encode.packvu32(len(arg)))
                if op_inner.endswith("idx"):
                    for a in arg:
                        f.write(encode.packvu32(a))
                elif op_inner == "valtype":
                    for a in arg:
                        assert a in ("i32", "i64", "f32", "f64", "v128", "funcref", "externref")
                        f.write(LANG_TYPES[a])
                else:
                    raise Exception("Wrong inner type for vec operand")
            elif op == "byte16":
                if type(arg) != int:
                    print(arg)
                assert type(arg) == int
                bytes_arr = arg.to_bytes(16, 'little')
                f.write(bytes_arr)
            elif op == "laneidx16":
                if type(arg) != int:
                    print(arg)
                assert type(arg) == int
                bytes_arr = arg.to_bytes(16, 'big')
                for b in bytes_arr:
                    f.write(encode.packvu8(b))
            elif op == "reftype":
                assert arg in ("funcref", "externref")
                f.write(LANG_TYPES[arg])
            elif op == "memarg":
                assert isinstance(arg, (tuple, list))
                assert len(arg) == 2
                f.write(encode.packvu32(arg[0]))
                f.write(encode.packvu32(arg[1]))
            elif op == "i32":
                assert isinstance(arg, int)
                if arg >= (1 << 31):
                    new_arg = arg - (1 << 32)
                else:
                    new_arg = arg
                f.write(encode.packvs32(new_arg))
            elif op == "i64":
                assert isinstance(arg, int)
                if arg >= (1 << 63):
                    new_arg = arg - (1 << 64)
                else:
                    new_arg = arg
                f.write(encode.packvs64(new_arg))
            elif op == "f32":
                assert isinstance(arg, float)
                f.write(encode.packf32(arg))
            elif op == "f64":
                assert isinstance(arg, float)
                f.write(encode.packf64(arg))
            elif op == "blocktype":
                assert isinstance(arg, (str, int))
                if isinstance(arg, str):
                    assert arg in ("emptyblock", "i32", "i64", "f32", "f64", "v128", "funcref", "externref")
                    f.write(LANG_TYPES[arg])
                else:
                    f.write(encode.packvs33(arg))
            else:
                raise Exception("Unknown operand")
        
        # TODO: check if there are nested instructions


# Collect field classes
_exportable_classes = WASMComponent, Function, ImportedFunction
__all__ = [name for name in globals()
           if isinstance(globals()[name], type) and issubclass(globals()[name], _exportable_classes)]
