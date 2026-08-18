#[cfg(feature = "wasm")]
use orca_wavebreak_macros::wasm_expose;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use std::fmt::{Debug, Formatter};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "Buffer")]
    pub type Bitmap;
}

#[cfg(feature = "wasm")]
impl Debug for Bitmap {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", JsValue::from(self))
    }
}

#[cfg(feature = "wasm")]
impl From<Bitmap> for [u8; 32] {
    fn from(bitmap: Bitmap) -> Self {
        let value = JsValue::from(bitmap);
        serde_wasm_bindgen::from_value(value).expect("Invalid bitmap")
    }
}

#[cfg(feature = "wasm")]
impl From<[u8; 32]> for Bitmap {
    fn from(bitmap: [u8; 32]) -> Self {
        Bitmap {
            obj: serde_wasm_bindgen::to_value(&bitmap).expect("Invalid bitmap"),
        }
    }
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn bitmap_add(bitmap: Bitmap, value: u8) -> Bitmap {
    let mut bitmap = bitmap.into();
    bitmap_set(&mut bitmap, value);
    bitmap.into()
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn bitmap_remove(bitmap: Bitmap, value: u8) -> Bitmap {
    let mut bitmap = bitmap.into();
    bitmap_unset(&mut bitmap, value);
    bitmap.into()
}

#[cfg(feature = "wasm")]
#[cfg_attr(feature = "wasm", wasm_expose)]
pub fn bitmap_has(bitmap: Bitmap, value: u8) -> bool {
    let bitmap = bitmap.into();
    bitmap_contains(&bitmap, value)
}

pub fn bitmap_set(bitmap: &mut [u8; 32], value: u8) {
    let byte_index = value / 8;
    let bit_index = value % 8;
    bitmap[byte_index as usize] |= 1 << bit_index;
}

pub fn bitmap_unset(bitmap: &mut [u8; 32], value: u8) {
    let byte_index = value / 8;
    let bit_index = value % 8;
    bitmap[byte_index as usize] &= !(1 << bit_index);
}

pub fn bitmap_contains(bitmap: &[u8; 32], value: u8) -> bool {
    let byte_index = value / 8;
    let bit_index = value % 8;
    bitmap[byte_index as usize] & (1 << bit_index) != 0
}

#[cfg(all(test, feature = "lib"))]
mod tests {
    use super::*;

    #[test]
    fn test_contains_bitmap() {
        let empty = [0u8; 32];
        let full = [255u8; 32];
        let odd_only = [85u8; 32];
        for i in 0..=255 {
            assert_eq!(bitmap_contains(&empty, i), false);
            assert_eq!(bitmap_contains(&full, i), true);
            assert_eq!(bitmap_contains(&odd_only, i), i & 1 == 0);
        }
    }

    #[test]
    fn test_set_bitmap() {
        let mut bitmap = [0u8; 32];
        for i in 0..=255 {
            if i & 1 == 0 {
                bitmap_set(&mut bitmap, i);
            }
        }
        assert_eq!(bitmap, [85u8; 32]);
    }

    #[test]
    fn test_unset_bitmap() {
        let mut bitmap = [255u8; 32];
        for i in 0..=255 {
            if i & 1 != 0 {
                bitmap_unset(&mut bitmap, i);
            }
        }
        assert_eq!(bitmap, [85u8; 32]);
    }
}
