#![allow(non_snake_case)]

#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

pub type CoreError = &'static str;

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const ARITHMETIC_OVERFLOW: CoreError = "Arithmetic over- or underflow";

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const AMOUNT_EXCEEDS_MAX_U64: CoreError = "Amount exceeds max u64";

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const AMOUNT_EXCEEDS_MAX_U128: CoreError = "Amount exceeds max u128";

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const BPS_EXCEEDS_MAX_U16: CoreError = "BPS exceeds 10000";

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const BEYOND_GRADUATION_TARGET: CoreError =
    "Results in a quote amount beyond graduation target";

#[cfg_attr(feature = "wasm", wasm_expose)]
pub const INVALID_PRICE_CURVE: CoreError = "Invalid price curve";
