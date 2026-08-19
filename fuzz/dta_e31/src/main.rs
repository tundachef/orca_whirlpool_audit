//! E3.1a–d — attribution + production update_tick vs bounded reference (host).
//!
//! Arms:
//!   A: rotate-only under production cast (short account + zero/canary past boundary)
//!   B: full production update_tick (rotate + bitmap + serialize)
//!   C: bounded reference (exact-sized ops, no MAX_LEN cast)
//!
//! Diffs:
//!   A−C  → runtime/OOB-specific rotate effect
//!   B−C  → whether serialize/repair leaves a lasting logical difference

use borsh::BorshSerialize;
use whirlpool::state::{
    DynamicTick, DynamicTickArray, DynamicTickArrayLoader, DynamicTickData, TickArrayType,
    TickUpdate,
};

const DISC: usize = 8;
const HEADER: usize = 52;
const START: i32 = 0;
const SPACING: u16 = 1;
const GUARD: usize = 16_384;
const N_AT: usize = DynamicTickArray::MIN_LEN + DynamicTickData::LEN; // 260

fn paint_gradient(body: &mut [u8]) {
    for (i, b) in body.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(3).wrapping_add(1);
    }
    // keep start_tick_index = 0
    body[0..4].copy_from_slice(&0i32.to_le_bytes());
}

fn expected_gradient(i: usize) -> u8 {
    (i as u8).wrapping_mul(3).wrapping_add(1)
}

fn init_update() -> TickUpdate {
    TickUpdate {
        initialized: true,
        liquidity_net: 111,
        liquidity_gross: 222,
        fee_growth_outside_a: 333,
        fee_growth_outside_b: 444,
        reward_growths_outside: [1, 2, 3],
    }
}

fn diff_count(a: &[u8], b: &[u8]) -> usize {
    let n = a.len().min(b.len());
    let mut c = a[..n].iter().zip(b[..n].iter()).filter(|(x, y)| x != y).count();
    c += a.len().abs_diff(b.len());
    c
}

fn first_diffs(a: &[u8], b: &[u8], max: usize) -> Vec<(usize, u8, u8)> {
    let mut out = Vec::new();
    for i in 0..a.len().min(b.len()) {
        if a[i] != b[i] {
            out.push((i, a[i], b[i]));
            if out.len() >= max {
                break;
            }
        }
    }
    out
}

/// Arm C — bounded reference: exact packed body, no MAX_LEN cast.
fn arm_c_reference() -> Vec<u8> {
    // After Uninit→Init first tick: account_len = 260, body = 252
    // Layout: header + I(113) + 87×U(1)
    let mut body = vec![0u8; N_AT - DISC];
    body[0..4].copy_from_slice(&START.to_le_bytes());
    body[36] = 0x01; // bitmap bit 0
    let tick = DynamicTick::from(&init_update());
    let enc = tick.try_to_vec().unwrap();
    assert_eq!(enc.len(), 113);
    body[HEADER..HEADER + enc.len()].copy_from_slice(&enc);
    // remaining uninit tags already 0
    body
}

/// Arm A — rotate-only under production cast; past-boundary = zeros (SVM-pad-like).
fn arm_a_rotate_only_zero_oob() -> Vec<u8> {
    let mut backing = vec![0u8; N_AT + GUARD]; // OOB zeros
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    paint_gradient(&mut backing[DISC..N_AT]);
    {
        let body = &mut backing[DISC..N_AT];
        let loader = DynamicTickArrayLoader::load_mut(body);
        // Mimic only the rotate part of update_tick Uninit→Init at offset 0
        let data_mut = {
            // tick_data_mut is private — use update_tick's rotate by calling update_tick
            // For arm A we need rotate WITHOUT serialize. Use unsafe cast like production:
            let raw = unsafe {
                &mut *(body.as_mut_ptr() as *mut [u8; DynamicTickArray::MAX_LEN])
            };
            &mut raw[HEADER..]
        };
        data_mut.rotate_right(DynamicTickData::LEN);
    }
    backing[DISC..N_AT].to_vec()
}

/// Arm A′ — rotate-only with non-zero canary past boundary (attribution of OOB contents).
fn arm_a_rotate_only_canary_oob() -> Vec<u8> {
    let mut backing = vec![0u8; N_AT + GUARD];
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    paint_gradient(&mut backing[DISC..N_AT]);
    for (i, b) in backing[N_AT..].iter_mut().enumerate() {
        *b = 0xE0u8.wrapping_add((i % 200) as u8);
    }
    {
        let body = &mut backing[DISC..N_AT];
        let raw = unsafe { &mut *(body.as_mut_ptr() as *mut [u8; DynamicTickArray::MAX_LEN]) };
        raw[HEADER..].rotate_right(DynamicTickData::LEN);
    }
    backing[DISC..N_AT].to_vec()
}

/// Arm B — full production update_tick on short account + zero OOB.
fn arm_b_production_update_tick() -> Vec<u8> {
    let mut backing = vec![0u8; N_AT + GUARD]; // zero OOB
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    // Empty DTA (all uninit), not gradient — matches real empty→init
    backing[DISC..N_AT].fill(0);
    backing[DISC..DISC + 4].copy_from_slice(&START.to_le_bytes());
    {
        let body = &mut backing[DISC..N_AT];
        let loader = DynamicTickArrayLoader::load_mut(body);
        loader
            .update_tick(START, SPACING, &init_update())
            .expect("update_tick");
    }
    backing[DISC..N_AT].to_vec()
}

/// Bounded-only rotate of the REAL tick-data region (200 bytes) — no OOB.
fn bounded_rotate_only_of_gradient() -> Vec<u8> {
    let mut body = vec![0u8; N_AT - DISC];
    paint_gradient(&mut body);
    let tick = &mut body[HEADER..]; // 200 bytes
    tick.rotate_right(DynamicTickData::LEN);
    body
}

fn main() {
    println!("E3.1a–d host differential (N_at_rotate={N_AT})");
    println!(
        "body={} claimed MAX_LEN={} beyond={}",
        N_AT - DISC,
        DynamicTickArray::MAX_LEN,
        DynamicTickArray::MAX_LEN - (N_AT - DISC)
    );

    let a_zero = arm_a_rotate_only_zero_oob();
    let a_canary = arm_a_rotate_only_canary_oob();
    let bounded_rot = bounded_rotate_only_of_gradient();
    let b = arm_b_production_update_tick();
    let c = arm_c_reference();

    println!("\n=== E3.1a attribution (rotate-only) ===");
    let a0_vs_bounded = diff_count(&a_zero, &bounded_rot);
    let ac_vs_bounded = diff_count(&a_canary, &bounded_rot);
    let a0_vs_ac = diff_count(&a_zero, &a_canary);
    println!("A(zero-OOB) vs bounded-rotate-only: Δ={a0_vs_bounded}");
    println!("A(canary-OOB) vs bounded-rotate-only: Δ={ac_vs_bounded}");
    println!("A(zero-OOB) vs A(canary-OOB): Δ={a0_vs_ac}");
    if a0_vs_bounded > 0 {
        println!(
            "  → zero-OOB rotate differs from bounded rotate — past-boundary zeros enter account"
        );
        println!("  first diffs A0 vs bounded: {:?}", first_diffs(&a_zero, &bounded_rot, 8));
    }
    if a0_vs_ac > 0 {
        println!(
            "  → canary OOB changes post-rotate account vs zero OOB — OOB CONTENTS attributable"
        );
        println!("  first diffs A0 vs Acanary: {:?}", first_diffs(&a_zero, &a_canary, 8));
    } else {
        println!("  → canary did not change account vs zero-OOB (unexpected if rotate spans past end)");
    }

    // How many of the first 112 tick bytes after rotate match canary pattern?
    let tick_start = HEADER;
    let mut canary_hits = 0usize;
    for i in 0..112 {
        let abs_body = tick_start + i;
        // After rotate_right(112) on claimed region, new[0..112] = old[claimed_len-112..]
        // claimed tick len = MAX_LEN - HEADER = 9952; last 112 of claimed are past real
        // (real tick=200), so they come from absolute account offsets past N_AT.
        if a_canary[abs_body] == 0xE0u8.wrapping_add((i % 200) as u8)
            || a_canary[abs_body] != expected_gradient(abs_body)
        {
            // check if equals canary from past boundary index
            // past-boundary index into GUARD: the last 112 of claimed tick_data map to
            // body offsets HEADER+9952-112 .. = beyond body; absolute past account.
            let past_i = i; // first 112 after rotate come from past end ordering
            let expect_canary = 0xE0u8.wrapping_add((past_i % 200) as u8);
            // Actually mapping is subtle — just report raw
            let _ = expect_canary;
            if a_canary[abs_body] != a_zero[abs_body] {
                canary_hits += 1;
            }
        }
    }
    println!("  bytes in first 112 tick slots where canary-OOB ≠ zero-OOB: {canary_hits}");

    println!("\n=== E3.1b/d production update_tick vs reference ===");
    let b_vs_c = diff_count(&b, &c);
    println!("B(production update_tick, zero-OOB) vs C(reference): Δ={b_vs_c}");
    if b_vs_c == 0 {
        println!(
            "  → FINAL logical body matches bounded reference after rotate+bitmap+serialize."
        );
        println!(
            "  → Intermediate OOB rotate (if any) was repaired by serialize under host semantics."
        );
    } else {
        println!("  → LASTING DIFF — program-visible corruption channel candidate");
        println!("  first diffs: {:?}", first_diffs(&b, &c, 16));
    }

    println!("\n=== E3.1 summary ===");
    println!(
        "rotate-only OOB attribution: {}",
        if a0_vs_ac > 0 || a0_vs_bounded > 0 {
            "YES — past-boundary participates in rotate-only account mutation"
        } else {
            "NO"
        }
    );
    println!(
        "full update_tick final vs reference: {}",
        if b_vs_c == 0 {
            "MATCH (repaired)"
        } else {
            "DIFFERS (investigate)"
        }
    );
    println!(
        "Terminology: rotate-only delta = post-execution account-data mutation candidate;"
    );
    println!("  not yet cross-transaction committed ledger state (E4).");
}
