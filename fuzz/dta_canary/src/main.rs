//! Phase A — production-path Layer-3 canary for DynamicTickArray.
//!
//! Uses the **actual** `whirlpool::state::DynamicTickArrayLoader::{load_mut, update_tick}`
//! (via `TickArrayType`), not a reimplementation of cast+rotate.
//!
//! Only intentional difference vs production: host `Vec` backing + gradient canary
//! past the supplied account-data boundary.
//!
//! Probes `N_at_rotate` explicitly along the production resize order.

use borsh::BorshSerialize;
use whirlpool::state::{
    DynamicTick, DynamicTickArray, DynamicTickArrayLoader, DynamicTickData, TickArrayType,
    TickUpdate, TICK_ARRAY_SIZE_USIZE,
};

const DISC: usize = 8;
const GUARD: usize = 16_384;
const CANARY: u8 = 0xA5;
const TICK_SPACING: u16 = 1;
const START_TICK_INDEX: i32 = 0;

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

fn report_dirty(label: &str, buf: &[u8], account_len: usize) {
    let n = count_dirty_past(buf, account_len);
    let first = first_dirty_past(buf, account_len);
    let last = last_dirty_past(buf, account_len);
    println!("  [{label}] account_len={account_len}");
    println!("  dirty past boundary: count={n} first={first:?} last={last:?}");
    if let (Some(f), Some(l)) = (first, last) {
        println!(
            "  canary detected modifications spanning {} bytes beyond account-data boundary",
            l - f + 1
        );
    } else {
        println!("  canary detected no past-boundary modifications");
    }
}

fn initialized_update() -> TickUpdate {
    TickUpdate {
        initialized: true,
        liquidity_net: 123,
        liquidity_gross: 456,
        fee_growth_outside_a: 678,
        fee_growth_outside_b: 901,
        reward_growths_outside: [234, 567, 890],
    }
}

fn uninitialized_update() -> TickUpdate {
    TickUpdate::default()
}

/// Write an empty DynamicTickArray body (all ticks Uninitialized) into account bytes.
fn write_empty_dta_account(backing: &mut [u8], account_len: usize) {
    assert!(account_len >= DynamicTickArray::MIN_LEN);
    assert!(backing.len() >= account_len);

    // Discriminator (Anchor DynamicTickArray) — value unused by loader (cast is on body).
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);

    let body = &mut backing[DISC..account_len];
    body.fill(0);
    // start_tick_index @ 0
    body[0..4].copy_from_slice(&START_TICK_INDEX.to_le_bytes());
    // whirlpool pubkey @ 4..36 already zero
    // tick_bitmap @ 36..52 already zero
    // ticks @ 52.. : MIN body has 88 uninit tags (zeros)
}

/// Write a one-initialized-tick account (first tick Init, rest Uninit) of size MIN+112.
fn write_one_tick_dta_account(backing: &mut [u8], account_len: usize) {
    assert_eq!(account_len, DynamicTickArray::MIN_LEN + DynamicTickData::LEN);
    write_empty_dta_account(backing, account_len);

    let body = &mut backing[DISC..account_len];
    // bitmap bit 0 set
    body[36] = 0x01;

    // tick 0 = Initialized(...) packed at tick data offset 52
    let tick_off = 52;
    let tick = DynamicTick::Initialized(DynamicTickData {
        liquidity_net: 123,
        liquidity_gross: 456,
        fee_growth_outside_a: 678,
        fee_growth_outside_b: 901,
        reward_growths_outside: [234, 567, 890],
    });
    let encoded = tick.try_to_vec().expect("encode DynamicTick");
    assert_eq!(encoded.len(), DynamicTick::INITIALIZED_LEN);
    body[tick_off..tick_off + encoded.len()].copy_from_slice(&encoded);
    // remaining 87 uninit tags already zero
}

/// Production increase order:
///   resize(+112) → load_mut → update_tick → rotate_right
fn exp_increase_first_tick_production_path() {
    println!("\n=== PHASE A: increase first tick (production update_tick, grow-then-rotate) ===");

    let min_len = DynamicTickArray::MIN_LEN;
    let data_len = DynamicTickData::LEN;
    let max_len = DynamicTickArray::MAX_LEN;

    println!("  constants: MIN_LEN={min_len} MAX_LEN={max_len} DynamicTickData::LEN={data_len}");
    println!("  TICK_ARRAY_SIZE={TICK_ARRAY_SIZE_USIZE}");

    // --- probe: before resize ---
    let n_before_resize = min_len;
    println!("  probe before_resize      data_len = {n_before_resize}");

    // --- simulate resize(+112) ---
    let n_after_resize = min_len + data_len; // 260
    println!("  probe after_resize       data_len = {n_after_resize}");
    assert_eq!(n_after_resize, 260);

    let mut backing = vec![0u8; n_after_resize + GUARD];
    write_empty_dta_account(&mut backing, n_after_resize);
    // Fresh resize bytes are zeros (already); paint canary past new boundary.
    paint_canary(&mut backing, n_after_resize);

    let n_before_load_mut = n_after_resize;
    println!("  probe before_load_mut    data_len = {n_before_load_mut}");

    let n_at_rotate = n_after_resize;
    println!("  probe N_at_rotate        data_len = {n_at_rotate}  (rotate inside update_tick)");

    // Production Anchor path: cast on data[8..]
    let body = &mut backing[DISC..n_at_rotate];
    println!(
        "  body.len()={} claimed Loader MAX_LEN={} overclaim={}",
        body.len(),
        max_len,
        max_len.saturating_sub(body.len())
    );

    let loader = DynamicTickArrayLoader::load_mut(body);
    assert_eq!(loader.start_tick_index(), START_TICK_INDEX);

    // Actual production transition: Uninit → Init at tick index 0
    loader
        .update_tick(START_TICK_INDEX, TICK_SPACING, &initialized_update())
        .expect("update_tick Uninit→Init");

    report_dirty("increase/rotate_right via production update_tick", &backing, n_at_rotate);
}

/// Production decrease order:
///   load_mut → update_tick → rotate_left → (later) resize(-112)
fn exp_decrease_last_tick_production_path() {
    println!("\n=== PHASE A: decrease last tick (production update_tick, rotate-then-shrink) ===");

    let n_at_rotate = DynamicTickArray::MIN_LEN + DynamicTickData::LEN; // still large
    println!("  probe N_at_rotate (pre-shrink) data_len = {n_at_rotate}");
    assert_eq!(n_at_rotate, 260);

    let mut backing = vec![0u8; n_at_rotate + GUARD];
    write_one_tick_dta_account(&mut backing, n_at_rotate);
    paint_canary(&mut backing, n_at_rotate);

    println!("  probe before_load_mut    data_len = {n_at_rotate}");
    println!("  probe before_update_tick data_len = {n_at_rotate}");

    let body = &mut backing[DISC..n_at_rotate];
    let loader = DynamicTickArrayLoader::load_mut(body);

    // Confirm starting state via production get_tick
    let before = loader
        .get_tick(START_TICK_INDEX, TICK_SPACING)
        .expect("get_tick before");
    assert!(before.initialized);

    loader
        .update_tick(START_TICK_INDEX, TICK_SPACING, &uninitialized_update())
        .expect("update_tick Init→Uninit");

    report_dirty("decrease/rotate_left via production update_tick", &backing, n_at_rotate);

    // Probe what resize(-112) would set afterward (not executed against canary here)
    let n_after_shrink = DynamicTickArray::MIN_LEN;
    println!("  probe after_resize(-112) would be data_len = {n_after_shrink}");
}

/// Full MAX account: Anchor body cast still overclaims by 8.
fn exp_full_account_anchor_plus8() {
    println!("\n=== PHASE A: full MAX account, Anchor data[8..] +8 overclaim ===");

    let account_len = DynamicTickArray::MAX_LEN;
    let mut backing = vec![0u8; account_len + GUARD];
    write_empty_dta_account(&mut backing, DynamicTickArray::MIN_LEN);
    // Fill rest of account with deterministic body bytes (all uninit layout doesn't fit MAX;
    // for this experiment we only need a valid header + enough zeros, then rotate on claimed extent).
    for i in DynamicTickArray::MIN_LEN..account_len {
        backing[i] = (i & 0xff) as u8;
    }
    // Ensure disc + header coherent; zero tick region would be ideal but MAX-sized all-uninit
    // is not a valid packed size. We only need rotate to run on the falsely extended view.
    // Re-init as empty header on full buffer: start/bitmap zero, ticks garbage OK for rotate probe.
    backing[DISC..account_len].fill(0);
    backing[DISC..DISC + 4].copy_from_slice(&START_TICK_INDEX.to_le_bytes());
    paint_canary(&mut backing, account_len);

    println!("  probe N_at_rotate        data_len = {account_len}");
    let body = &mut backing[DISC..account_len];
    println!(
        "  body.len()={} MAX_LEN={} overclaim={}",
        body.len(),
        DynamicTickArray::MAX_LEN,
        DynamicTickArray::MAX_LEN - body.len()
    );

    let loader = DynamicTickArrayLoader::load_mut(body);
    loader
        .update_tick(START_TICK_INDEX, TICK_SPACING, &initialized_update())
        .expect("update_tick on full account");

    report_dirty("full Anchor cast rotate via production update_tick", &backing, account_len);
}

fn print_fidelity_table() {
    println!("\n=== Harness fidelity ===");
    println!("Component                         Production     Harness");
    println!("---------------------------------------------------------");
    println!("DynamicTickArrayLoader            same           same (whirlpool path dep)");
    println!("update_tick                       same           same");
    println!("tick_data_mut                     same           same (via update_tick)");
    println!("rotate_right/left                 same           same");
    println!("account body offset               8              8");
    println!("account length (first-tick)       260            260");
    println!("resize ordering                   same           same (simulated)");
    println!("tick state transition             same           same");
    println!("backing allocation                SVM            host+canary  ← only intentional diff");
}

fn main() {
    println!("dta_canary Phase A — production-path canary");
    println!(
        "MIN_LEN={} MAX_LEN={} INIT_LEN={} UNINIT_LEN={} DATA_LEN={}",
        DynamicTickArray::MIN_LEN,
        DynamicTickArray::MAX_LEN,
        DynamicTick::INITIALIZED_LEN,
        DynamicTick::UNINITIALIZED_LEN,
        DynamicTickData::LEN,
    );
    print_fidelity_table();
    exp_increase_first_tick_production_path();
    exp_decrease_last_tick_production_path();
    exp_full_account_anchor_plus8();
    println!("\n=== DONE ===");
}
