//! Model for types in the WebAssembly syntax.

use enum_iterator::Sequence;

/// Number types classify numeric values.
/// Number types are transparent, meaning that their bit patterns can be observed.
/// Values of number type can be stored in memories.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#number-types>
///
/// # Examples
/// ```rust
/// use wasm_ast::{ValueType, NumberType};
///
/// assert_eq!(ValueType::I32, NumberType::I32.into());
/// assert_eq!(ValueType::I64, NumberType::I64.into());
/// assert_eq!(ValueType::F32, NumberType::F32.into());
/// assert_eq!(ValueType::F64, NumberType::F64.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum NumberType {
    I32,
    I64,
    F32,
    F64,
}

impl From<IntegerType> for NumberType {
    fn from(kind: IntegerType) -> Self {
        match kind {
            IntegerType::I32 => NumberType::I32,
            IntegerType::I64 => NumberType::I64,
        }
    }
}

impl From<FloatType> for NumberType {
    fn from(kind: FloatType) -> Self {
        match kind {
            FloatType::F32 => NumberType::F32,
            FloatType::F64 => NumberType::F64,
        }
    }
}

/// The types 𝗂𝟥𝟤 and 𝗂𝟨𝟦 classify 32 and 64 bit integers, respectively.
/// Integers are not inherently signed or unsigned, their interpretation is determined by individual operations.
///
/// # Examples
/// ```rust
/// use wasm_ast::{ValueType, NumberType, IntegerType};
///
/// assert_eq!(ValueType::I32, IntegerType::I32.into());
/// assert_eq!(NumberType::I32, IntegerType::I32.into());
/// assert_eq!(ValueType::I64, IntegerType::I64.into());
/// assert_eq!(NumberType::I64, IntegerType::I64.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum IntegerType {
    I32,
    I64,
}

/// The types 𝖿𝟥𝟤 and 𝖿𝟨𝟦 classify 32 and 64 bit floating-point data, respectively.
/// They correspond to the respective binary floating-point representations,
/// also known as single and double precision, as defined by the IEEE 754-2019 standard (Section 3.3).
///
/// # Examples
/// ```rust
/// use wasm_ast::{ValueType, NumberType, FloatType};
///
/// assert_eq!(ValueType::F32, FloatType::F32.into());
/// assert_eq!(NumberType::F32, FloatType::F32.into());
/// assert_eq!(ValueType::F64, FloatType::F64.into());
/// assert_eq!(NumberType::F64, FloatType::F64.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum FloatType {
    F32,
    F64,
}

/// Vector types (v128)
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum VectorType {
    V128,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum IShape {
    I8X16,
    I16X8,
    I32X4,
    I64X2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum FShape {
    F32X4,
    F64X2,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum VectorShape {
    I8X16,
    I16X8,
    I32X4,
    I64X2,
    F32X4,
    F64X2,
}

impl From<IShape> for VectorShape {
    fn from(kind: IShape) -> Self {
        match kind {
            IShape::I8X16 => VectorShape::I8X16,
            IShape::I16X8 => VectorShape::I16X8,
            IShape::I32X4 => VectorShape::I32X4,
            IShape::I64X2 => VectorShape::I64X2,
        }
    }
}

impl From<FShape> for VectorShape {
    fn from(kind: FShape) -> Self {
        match kind {
            FShape::F32X4 => VectorShape::F32X4,
            FShape::F64X2 => VectorShape::F64X2,
        }
    }
}

pub type LaneIdx = u8;

/// Reference types classify first-class references to objects in the runtime store.
/// The type 𝖿𝗎𝗇𝖼𝗋𝖾𝖿 denotes the infinite union of all references to functions,
/// regardless of their function types.
/// The type 𝖾𝗑𝗍𝖾𝗋𝗇𝗋𝖾𝖿 denotes the infinite union of all references to objects owned by the
/// embedder and that can be passed into WebAssembly under this type.
/// Reference types are opaque, meaning that neither their size nor their bit pattern can be observed.
/// Values of reference type can be stored in tables.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#reference-types>
///
/// # Examples
/// ```rust
/// use wasm_ast::{ValueType, ReferenceType};
///
/// assert_eq!(ValueType::FunctionReference, ReferenceType::Function.into());
/// assert_eq!(ValueType::ExternalReference, ReferenceType::External.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum ReferenceType {
    Function,
    External,
}

/// Value types classify the individual values that WebAssembly code can compute with and the values that a variable accepts.
/// They are either number types or reference types.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#value-types>
///
/// # Examples
/// ```rust
/// use wasm_ast::{ValueType, ReferenceType, IntegerType, FloatType, NumberType};
///
/// assert_eq!(ValueType::I32, IntegerType::I32.into());
/// assert_eq!(ValueType::I32, NumberType::I32.into());
/// assert_eq!(ValueType::I64, IntegerType::I64.into());
/// assert_eq!(ValueType::I64, NumberType::I64.into());
/// assert_eq!(ValueType::F32, FloatType::F32.into());
/// assert_eq!(ValueType::F32, NumberType::F32.into());
/// assert_eq!(ValueType::F64, FloatType::F64.into());
/// assert_eq!(ValueType::F64, NumberType::F64.into());
/// assert_eq!(ValueType::FunctionReference, ReferenceType::Function.into());
/// assert_eq!(ValueType::ExternalReference, ReferenceType::External.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq, Sequence)]
pub enum ValueType {
    I32,
    I64,
    F32,
    F64,
    V128,
    FunctionReference,
    ExternalReference,
}

impl<T> From<T> for ValueType
where
    T: Into<NumberType>,
{
    fn from(kind: T) -> Self {
        match kind.into() {
            NumberType::I32 => ValueType::I32,
            NumberType::I64 => ValueType::I64,
            NumberType::F32 => ValueType::F32,
            NumberType::F64 => ValueType::F64,
        }
    }
}

impl From<VectorType> for ValueType
{
    fn from(kind: VectorType) -> Self {
        match kind.into() {
            VectorType::V128 => ValueType::V128,
        }
    }
}

impl From<ReferenceType> for ValueType {
    fn from(kind: ReferenceType) -> Self {
        match kind {
            ReferenceType::Function => ValueType::FunctionReference,
            ReferenceType::External => ValueType::ExternalReference,
        }
    }
}

/// Result types classify the result of executing instructions or functions,
/// which is a sequence of values, written with brackets.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#result-types>
///
/// # Examples
///
/// ## Empty
/// ```rust
/// use wasm_ast::ResultType;
///
/// let result_type = ResultType::empty();
///
/// assert_eq!(result_type.len(), 0);
/// assert!(result_type.is_empty());
/// assert_eq!(result_type.kinds(), &[]);
/// ```
///
/// ## Non-Empty
/// ```rust
/// use wasm_ast::{ResultType, IntegerType, FloatType, ReferenceType, ValueType};
///
/// let result_type = ResultType::new(vec![
///     IntegerType::I32.into(),
///     IntegerType::I64.into(),
///     FloatType::F32.into(),
///     FloatType::F64.into(),
///     ReferenceType::Function.into(),
///     ReferenceType::External.into(),
/// ]);
///
/// assert_eq!(result_type.len(), 6);
/// assert!(!result_type.is_empty());
/// assert_eq!(
///     result_type.kinds(),
///     &[
///         ValueType::I32,
///         ValueType::I64,
///         ValueType::F32,
///         ValueType::F64,
///         ValueType::FunctionReference,
///         ValueType::ExternalReference,
///     ]
/// );
/// assert_eq!(
///     result_type,
///     vec![
///         ValueType::I32,
///         ValueType::I64,
///         ValueType::F32,
///         ValueType::F64,
///         ValueType::FunctionReference,
///         ValueType::ExternalReference,
///     ].into()
/// );
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultType {
    kinds: Vec<ValueType>,
}

impl ResultType {
    /// Creates a new `ResultType` with the given value types.
    pub fn new(kinds: Vec<ValueType>) -> Self {
        ResultType { kinds }
    }

    /// Creates a new empty `ResultType`.
    pub fn empty() -> Self {
        ResultType { kinds: vec![] }
    }

    /// A reference to a slice of the `ValueType`s.
    pub fn kinds(&self) -> &[ValueType] {
        &self.kinds
    }

    /// The length of the `ValueType` vector.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    /// Returns true if this `ResultType` has a length of zero, false otherwise.
    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }
}

impl From<Vec<ValueType>> for ResultType {
    fn from(kinds: Vec<ValueType>) -> Self {
        ResultType { kinds }
    }
}

/// Function types classify the signature of functions,
/// mapping a vector of parameters to a vector of results.
/// They are also used to classify the inputs and outputs of instructions
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#function-types>
///
/// # Examples
///
/// ## Input & Output
/// ```rust
/// use wasm_ast::{FunctionType, ResultType};
///
/// let function_type = FunctionType::new(ResultType::empty(), ResultType::empty());
///
/// assert!(function_type.parameters().is_empty());
/// assert!(function_type.results().is_empty());
/// ```
///
/// ## Input Only
/// ```rust
/// use wasm_ast::{FunctionType, ResultType, ValueType};
///
/// let function_type = FunctionType::side_effect(ResultType::from(vec![ValueType::I32]));
///
/// assert_eq!(function_type.parameters().kinds(), &[ValueType::I32]);
/// assert!(function_type.results().is_empty());
/// ```
///
/// ## Output Only
/// ```rust
/// use wasm_ast::{FunctionType, ResultType, ValueType};
///
/// let function_type = FunctionType::nullary(ResultType::from(vec![ValueType::I32]));
///
/// assert!(function_type.parameters().is_empty());
/// assert_eq!(function_type.results().kinds(), &[ValueType::I32]);
/// ```
///
/// ## No Input or Output
/// ```rust
/// use wasm_ast::{FunctionType, ResultType, ValueType};
///
/// let function_type = FunctionType::runnable();
///
/// assert!(function_type.parameters().is_empty());
/// assert!(function_type.results().is_empty());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionType {
    parameters: ResultType,
    results: ResultType,
}

impl FunctionType {
    /// Creates a new function signature with the given parameter and result types.
    pub fn new(parameters: ResultType, results: ResultType) -> Self {
        FunctionType {
            parameters,
            results,
        }
    }

    /// Creates a new function signature with the given parameter types and no result types.
    pub fn side_effect(parameters: ResultType) -> Self {
        FunctionType {
            parameters,
            results: ResultType::empty(),
        }
    }

    /// Creates a new function signature with the given result types and no parameter types.
    pub fn nullary(results: ResultType) -> Self {
        FunctionType {
            parameters: ResultType::empty(),
            results,
        }
    }

    /// Creates a new function signature with the no parameter or result types.
    pub fn runnable() -> Self {
        FunctionType {
            parameters: ResultType::empty(),
            results: ResultType::empty(),
        }
    }

    /// The parameter types of this `FunctionType`.
    pub fn parameters(&self) -> &ResultType {
        &self.parameters
    }

    /// The result types of this `FunctionType`.
    pub fn results(&self) -> &ResultType {
        &self.results
    }
}

/// Limits classify the size range of resizeable storage associated with memory types and table types.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#limits>
///
/// # Examples
///
/// ## New
/// ```rust
/// use wasm_ast::Limit;
///
/// let max = Some(2);
/// let min = 0;
/// let limit = Limit::new(min, max);
///
/// assert_eq!(limit.min(), min);
/// assert_eq!(limit.max(), max);
/// ```
///
/// ## Unbounded
/// ```rust
/// use wasm_ast::Limit;
///
/// assert_eq!(Limit::unbounded(2), Limit::new(2, None));
/// ```
///
/// /// ## Unbounded
/// ```rust
/// use wasm_ast::Limit;
///
/// assert_eq!(Limit::bounded(2, 5), Limit::new(2, Some(5)));
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Limit {
    min: u32,
    max: Option<u32>,
}

impl Limit {
    /// Creates a new limit with a required minimum and optional maximum.
    pub fn new(min: u32, max: Option<u32>) -> Self {
        Limit { min, max }
    }

    /// Creates a new limit with a required minimum and no maximum.
    pub fn unbounded(min: u32) -> Self {
        Limit { min, max: None }
    }

    /// Creates a new limit with a required minimum and maximum.
    pub fn bounded(min: u32, max: u32) -> Self {
        Limit {
            min,
            max: Some(max),
        }
    }

    /// The minimum value of the limit.
    pub fn min(&self) -> u32 {
        self.min
    }

    /// The optional maximum value of the limit.
    pub fn max(&self) -> Option<u32> {
        self.max
    }
}

/// Memory types classify linear memories and their size range.
/// The limits constrain the minimum and optionally the maximum size of a memory.
/// The limits are given in units of page size.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#memory-types>
///
/// # Examples
/// ```rust
/// use wasm_ast::{Limit, MemoryType};
///
/// let limit = Limit::unbounded(0);
/// let memory_type = MemoryType::new(limit.clone());
///
/// assert_eq!(memory_type.limits(), &limit);
/// assert_eq!(memory_type, limit.into());
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct MemoryType {
    limits: Limit,
}

impl MemoryType {
    /// Creates a new memory type from the given limits.
    pub fn new(limit: Limit) -> Self {
        MemoryType { limits: limit }
    }

    /// The limits of the number of pages for this `MemoryType`.
    pub fn limits(&self) -> &Limit {
        &self.limits
    }
}

impl From<Limit> for MemoryType {
    fn from(limit: Limit) -> Self {
        MemoryType { limits: limit }
    }
}

/// Table types classify tables over elements of reference type within a size range.
/// Like memories, tables are constrained by limits for their minimum and optionally maximum size.
/// The limits are given in numbers of entries.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#table-types>
///
/// # Examples
/// ```rust
/// use wasm_ast::{Limit, TableType, ReferenceType};
///
/// let limit = Limit::unbounded(0);
/// let table_type = TableType::new( ReferenceType::External,limit.clone());
///
/// assert_eq!(table_type.limits(), &limit);
/// assert_eq!(table_type.kind(), ReferenceType::External);
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TableType {
    limits: Limit,
    kind: ReferenceType,
}

impl TableType {
    /// Creates a new `TableType` for the given limits and reference type.
    pub fn new(kind: ReferenceType, limits: Limit) -> Self {
        TableType { limits, kind }
    }

    /// The limits of the number of elements for this `TableType`.
    pub fn limits(&self) -> &Limit {
        &self.limits
    }

    /// The reference type of the elements of this `TableType`.
    pub fn kind(&self) -> ReferenceType {
        self.kind
    }
}

/// Global types classify global variables, which hold a value and can either be mutable or immutable.
///
/// See <https://webassembly.github.io/spec/core/syntax/types.html#global-types>
///
/// # Examples
/// ## Mutable
/// ```rust
/// use wasm_ast::{ValueType, GlobalType, Mutability};
///
/// let mutable = GlobalType::mutable(ValueType::I64);
///
/// assert_eq!(mutable.mutability(), Mutability::Mutable);
/// assert_eq!(mutable.kind(), ValueType::I64);
/// assert_eq!(mutable, GlobalType::new( ValueType::I64,Mutability::Mutable));
/// ```
///
/// ## Immutable
/// ```rust
/// use wasm_ast::{ValueType, GlobalType, Mutability};
///
/// let immutable = GlobalType::immutable(ValueType::F64);
///
/// assert_eq!(immutable.mutability(), Mutability::Immutable);
/// assert_eq!(immutable.kind(), ValueType::F64);
/// assert_eq!(immutable, GlobalType::new( ValueType::F64,Mutability::Immutable));
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct GlobalType {
    mutability: Mutability,
    kind: ValueType,
}

impl GlobalType {
    /// Creates a new `GlobalType` for a global variable with the given mutability and value type.
    pub fn new(kind: ValueType, mutability: Mutability) -> Self {
        GlobalType { mutability, kind }
    }

    /// Creates a new `GlobalType` for a mutable global variable.
    pub fn mutable(kind: ValueType) -> Self {
        GlobalType {
            mutability: Mutability::Mutable,
            kind,
        }
    }

    /// Creates a new `GlobalType` for an immutable (i.e. constant) global variable.
    pub fn immutable(kind: ValueType) -> Self {
        GlobalType {
            mutability: Mutability::Immutable,
            kind,
        }
    }

    /// The `ValueType` of the global variable defined by this `GlobalType`.
    pub fn kind(&self) -> ValueType {
        self.kind
    }

    /// The mutability (i.e. variable versus constant) of the global variable type.
    pub fn mutability(&self) -> Mutability {
        self.mutability
    }
}

/// The mutability of a global variable.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Mutability {
    Mutable,
    Immutable,
}
