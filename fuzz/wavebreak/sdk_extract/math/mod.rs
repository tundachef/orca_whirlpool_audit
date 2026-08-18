#![cfg_attr(feature = "wasm", allow(non_snake_case))]

pub mod bitmap;
pub mod curve;
pub mod error;
pub mod fee;
pub mod mcap;
pub mod price;
pub mod quote;
pub mod token;

#[cfg(feature = "wasm")]
use serde::Serializer;

// Serialize a u64 as a u128. This is so that we can use u64 value in rust
// but serialize as a bigint in wasm.
#[cfg(feature = "wasm")]
pub fn u64_serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u128(*value as u128)
}

// While `wasm_expose` doesn't automatically convert rust `u128` to js `bigint`, we have
// to proxy it through an opaque type that we define here. This is a workaround until
// `wasm_bindgen` supports `u128` abi conversion natively.

#[cfg(not(feature = "wasm"))]
pub type U128 = u128;

#[cfg(feature = "wasm")]
use core::fmt::{Debug, Formatter};

#[cfg(feature = "wasm")]
use ethnum::U256;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "bigint")]
    pub type U128;
}

#[cfg(feature = "wasm")]
impl Debug for U128 {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", JsValue::from(self))
    }
}

#[cfg(feature = "wasm")]
impl From<U128> for u128 {
    fn from(value: U128) -> u128 {
        JsValue::from(value)
            .try_into()
            .expect("Number too large to fit in u128")
    }
}

#[cfg(feature = "wasm")]
impl From<U128> for U256 {
    fn from(value: U128) -> U256 {
        let u_128: u128 = value.into();
        <U256>::from(u_128)
    }
}

#[cfg(feature = "wasm")]
impl From<u128> for U128 {
    fn from(value: u128) -> U128 {
        JsValue::from(value).unchecked_into()
    }
}

#[cfg(feature = "wasm")]
impl TryFrom<U256> for U128 {
    type Error = core::num::TryFromIntError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let u_128: u128 = value.try_into()?;
        Ok(U128::from(u_128))
    }
}

#[cfg(feature = "wasm")]
impl PartialEq<u128> for U128 {
    fn eq(&self, other: &u128) -> bool {
        self == &(*other).into()
    }
}
