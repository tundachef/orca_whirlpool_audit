//! Layer-3 instrumentation for DynamicTickArrayLoader size lie.
//! No Agave — proves which bytes past the real account body are actually touched
//! when rotate_* runs on a falsely sized MAX_LEN view.
//!
//! Mirrors production constants and the unsafe cast + rotate pattern.

use std::mem::size_of;

const DISC: usize = 8;
const HEADER: usize = 4 + 32 + 16; // start + whirlpool + bitmap
const TICK_DATA_OFFSET: usize = HEADER; // 52 in body coords / loader without disc... 
// Anchor loader is cast on data[8..], so loader[0] = body[0], TICK_DATA_OFFSET in loader = 52
const TICK_DATA_OFF: usize = 52;
const UNINIT: usize = 1;
const INIT: usize = 113;
const DATA_LEN: usize = 112; // DynamicTickData::LEN
const N_TICKS: usize = 88;
const MAX_LEN: usize = DISC + HEADER + INIT * N_TICKS; // 10004
const MIN_LEN: usize = DISC + HEADER + UNINIT * N_TICKS; // 148
const CANARY: u8 = 0xA5;
const GUARD: usize = 16_384;

#[repr(C)]
struct Loader([u8; MAX_LEN]);

/// Mimic `load_mut(&mut data[8..])` — discards slice length.
unsafe fn load_mut_body(body: &mut [u8]) -> &mut Loader {
    &mut *(body.as_mut_ptr() as *mut Loader)
}

fn tick_data_mut(loader: &mut Loader) -> &mut [u8] {
    &mut loader.0[TICK_DATA_OFF..]
}

/// Gradient canary: each past-boundary byte is unique so rotate canary↔canary still shows.
fn paint_canary(buf: &mut [u8], account_len: usize) {
    for (i, b) in buf[account_len..].iter_mut().enumerate() {
        *b = ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY);
    }
}

fn expected_canary(account_len: usize, abs_index: usize) -> u8 {
    let i = abs_index - account_len;
    ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY)
}

fn first_dirty_past(buf: &[u8], account_len: usize) -> Option<usize> {
    (account_len..buf.len()).find(|&i| buf[i] != expected_canary(account_len, i))
}

fn last_dirty_past(buf: &[u8], account_len: usize) -> Option<usize> {
    (account_len..buf.len())
        .rev()
        .find(|&i| buf[i] != expected_canary(account_len, i))
}

fn count_dirty_past(buf: &[u8], account_len: usize) -> usize {
    (account_len..buf.len())
        .filter(|&i| buf[i] != expected_canary(account_len, i))
        .count()
}

/// Experiment: rotate_right on short body, mimicking increase AFTER +112 resize.
fn exp_rotate_right_after_increase() {
    println!("\n=== EXP B/C: rotate_right after +112 (increase path order) ===");
    // Production increase: resize first, then rotate.
    // Start empty account MIN_LEN, resize to MIN+112, body = MIN_LEN-8+112 = 252
    let account_len = MIN_LEN + DATA_LEN; // 260 full account with disc
    let body_len = account_len - DISC; // 252
    let mut backing = vec![0u8; account_len + GUARD];
    // disc + empty body pattern
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    // body already zeroed; after resize the extra 112 are zero (fresh)
    paint_canary(&mut backing, account_len);

    let body = &mut backing[DISC..account_len];
    assert_eq!(body.len(), body_len);

    unsafe {
        let loader = load_mut_body(body);
        let data_mut = tick_data_mut(loader);
        let byte_offset = 0usize;
        let shift = &mut data_mut[byte_offset..];
        println!(
            "  claimed shift_data.len() = {} (type-level)",
            shift.len()
        );
        println!(
            "  real tick bytes available in account body = {}",
            body_len.saturating_sub(TICK_DATA_OFF)
        );
        shift.rotate_right(DATA_LEN);
    }

    let first = first_dirty_past(&backing, account_len);
    let last = last_dirty_past(&backing, account_len);
    let n = count_dirty_past(&backing, account_len);
    println!("  account_len (with disc) = {account_len}");
    println!("  dirty past boundary: count={n} first={first:?} last={last:?}");
    if let (Some(f), Some(l)) = (first, last) {
        println!("  measured OOB span = {} bytes (inclusive)", l - f + 1);
    }
}

/// Decrease path: rotate_left WHILE still large, then would shrink.
fn exp_rotate_left_before_decrease() {
    println!("\n=== EXP B/C: rotate_left before -112 (decrease path order) ===");
    // One initialized tick packed: size = MIN + 112
    let account_len = MIN_LEN + DATA_LEN; // 260
    let body_len = account_len - DISC;
    let mut backing = vec![0u8; account_len + GUARD];
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    // Put a plausible initialized tick at offset 0 in tick region: tag=1 + 112 data
    let tick_start = DISC + TICK_DATA_OFF;
    backing[tick_start] = 1;
    for i in 0..DATA_LEN {
        backing[tick_start + 1 + i] = (i & 0xff) as u8;
    }
    // remaining uninit ticks as zeros (87 bytes) — already zero
    // bitmap: bit 0 set
    let bitmap_off = DISC + 36;
    backing[bitmap_off] = 0x01;

    paint_canary(&mut backing, account_len);

    let body = &mut backing[DISC..account_len];
    unsafe {
        let loader = load_mut_body(body);
        let data_mut = tick_data_mut(loader);
        let shift = &mut data_mut[0..];
        println!("  claimed shift_data.len() = {}", shift.len());
        shift.rotate_left(DATA_LEN);
    }

    let first = first_dirty_past(&backing, account_len);
    let last = last_dirty_past(&backing, account_len);
    let n = count_dirty_past(&backing, account_len);
    println!("  account_len = {account_len}");
    println!("  dirty past boundary: count={n} first={first:?} last={last:?}");
    if let (Some(f), Some(l)) = (first, last) {
        println!("  measured OOB span = {} bytes", l - f + 1);
    }
}

/// Full account still has +8 overclaim on Anchor body cast.
fn exp_full_account_plus8() {
    println!("\n=== EXP: full MAX account, Anchor data[8..] still +8 overclaim ===");
    let account_len = MAX_LEN;
    let body_len = account_len - DISC; // 9996
    let mut backing = vec![0u8; account_len + GUARD];
    for i in DISC..account_len {
        backing[i] = (i & 0xff) as u8;
    }
    paint_canary(&mut backing, account_len);

    let body = &mut backing[DISC..account_len];
    assert_eq!(body.len(), body_len);
    println!(
        "  body.len()={} MAX_LEN={} overclaim={}",
        body_len,
        MAX_LEN,
        MAX_LEN - body_len
    );

    unsafe {
        let loader = load_mut_body(body);
        let data_mut = tick_data_mut(loader);
        // Small offset so claimed_shift_len > 112
        let byte_offset = 0usize;
        let shift = &mut data_mut[byte_offset..];
        println!(
            "  byte_offset={byte_offset} claimed_shift_len={}",
            shift.len()
        );
        // Only form the slice — don't rotate if len < DATA_LEN (shouldn't happen)
        if shift.len() >= DATA_LEN {
            shift.rotate_right(DATA_LEN);
        }
    }

    let n = count_dirty_past(&backing, account_len);
    let first = first_dirty_past(&backing, account_len);
    let last = last_dirty_past(&backing, account_len);
    println!("  dirty past boundary: count={n} first={first:?} last={last:?}");
}

/// get_tick-style slice construction: does forming [off..off+113] require claimed length?
fn exp_get_tick_slice() {
    println!("\n=== EXP A/P2: get_tick slice construction (Layer 2 vs Layer 3) ===");
    println!("  Source: `let mut tick_data = &ticks_data[byte_offset..byte_offset + 113];`");
    println!("  Then DynamicTick::deserialize(&mut tick_data).");
    println!("  Borsh enum: Uninitialized reads 1 byte tag; Initialized reads 113.");
    println!("  BUT forming the 113-byte slice indexes the claimed tick_data slice (len=9952).");
    println!("  Layer 2: slice construction is in-bounds of CLAIMED len whenever byte_offset+113 <= 9952.");
    println!("  Layer 3: actual deserialize READ is 1 byte (uninit) or 113 (init) from that slice.");
    println!("  ⇒ P2 rewritten: unconditional 113-byte *slice formation* on claimed extent;");
    println!("    not necessarily a 113-byte *physical read* for uninitialized ticks.");
}

fn main() {
    println!("MAX_LEN={MAX_LEN} MIN_LEN={MIN_LEN} size_of::<Loader>()={}", size_of::<Loader>());
    println!("align_of::<Loader>()={}", std::mem::align_of::<Loader>());
    exp_get_tick_slice();
    exp_rotate_right_after_increase();
    exp_rotate_left_before_decrease();
    exp_full_account_plus8();
    println!("\n=== DONE ===");
}
