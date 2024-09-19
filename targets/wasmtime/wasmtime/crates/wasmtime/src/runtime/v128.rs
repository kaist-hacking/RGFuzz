#![cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    allow(unused_imports)
)]

use crate::store::StoreOpaque;
use crate::{ValRaw, ValType, WasmTy};
use std::cmp::Ordering;
use std::fmt;
use wasmtime_runtime::V128Abi;

/// Representation of a 128-bit vector type, `v128`, for WebAssembly.
///
/// This type corresponds to the `v128` type in WebAssembly and can be used with
/// the [`TypedFunc`] API for example. This is additionally
/// the payload of [`Val::V128`](crate::Val).
///
/// # Platform specifics
///
/// This type can currently only be used on x86_64 and AArch64 with the
/// [`TypedFunc`] API. Rust does not have stable support on other platforms for
/// this type so invoking functions with `v128` parameters requires the
/// [`Func::call`](crate::Func::call) API (or perhaps
/// [`Func::call_unchecked`](crate::Func::call_unchecked).
///
/// [`TypedFunc`]: crate::TypedFunc
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct V128(V128Abi);

union Reinterpret {
    abi: V128Abi,
    u128: u128,
}

impl V128 {
    /// Returns the representation of this `v128` as a 128-bit integer in Rust.
    pub fn as_u128(&self) -> u128 {
        unsafe { Reinterpret { abi: self.0 }.u128 }
    }
}

/// Primary constructor of a `V128` type.
impl From<u128> for V128 {
    fn from(val: u128) -> V128 {
        unsafe { V128(Reinterpret { u128: val }.abi) }
    }
}

impl From<V128> for u128 {
    fn from(val: V128) -> u128 {
        val.as_u128()
    }
}

impl fmt::Debug for V128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_u128().fmt(f)
    }
}

impl PartialEq for V128 {
    fn eq(&self, other: &V128) -> bool {
        self.as_u128() == other.as_u128()
    }
}

impl Eq for V128 {}

impl PartialOrd for V128 {
    fn partial_cmp(&self, other: &V128) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for V128 {
    fn cmp(&self, other: &V128) -> Ordering {
        self.as_u128().cmp(&other.as_u128())
    }
}

// Note that this trait is conditionally implemented which is intentional. See
// the documentation above in the `cfg_if!` for why this is conditional.
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
unsafe impl WasmTy for V128 {
    type Abi = V128Abi;

    #[inline]
    fn valtype() -> ValType {
        ValType::V128
    }

    #[inline]
    fn compatible_with_store(&self, _: &StoreOpaque) -> bool {
        true
    }

    #[inline]
    fn is_externref(&self) -> bool {
        false
    }

    #[inline]
    unsafe fn abi_from_raw(raw: *mut ValRaw) -> Self::Abi {
        V128::from((*raw).get_v128()).0
    }

    #[inline]
    unsafe fn abi_into_raw(abi: Self::Abi, raw: *mut ValRaw) {
        *raw = ValRaw::v128(V128(abi).as_u128());
    }

    #[inline]
    fn into_abi(self, _store: &mut StoreOpaque) -> Self::Abi {
        self.0
    }

    #[inline]
    unsafe fn from_abi(abi: Self::Abi, _store: &mut StoreOpaque) -> Self {
        V128(abi)
    }
}
