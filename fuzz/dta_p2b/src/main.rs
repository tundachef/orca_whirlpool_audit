//! P2b — can `byte_offset + 113` exceed the REAL tick-data length while the
//! unsafe MAX_LEN view still treats the slice as in-bounds, and can Borsh
//! then physically read 113 bytes across the boundary?
//!
//! Under the packed-layout invariant (bitmap ↔ sizes), every Initialized tick
//! occupies a full 113-byte record inside the packed region, so
//! `byte_offset(i) + 113 <= packed_len` for every Initialized i.
//!
//! Uninitialized ticks near the end still *form* a 113-byte slice (P2a) that
//! may extend past the real end, but Borsh only reads 1 byte (tag).

use borsh::{BorshDeserialize, BorshSerialize};
use whirlpool::state::{
    DynamicTick, DynamicTickArray, DynamicTickArrayLoader, TickArrayType, TickUpdate,
    TICK_ARRAY_SIZE_USIZE,
};

const DISC: usize = 8;
const HEADER: usize = 52;
const N: usize = TICK_ARRAY_SIZE_USIZE;
const START: i32 = 0;
const SPACING: u16 = 1;
const GUARD: usize = 4096;
const CANARY: u8 = 0xA5;

#[derive(Clone, Copy)]
enum Cell {
    U,
    I,
}

impl Cell {
    fn size(self) -> usize {
        match self {
            Cell::U => 1,
            Cell::I => 113,
        }
    }
}

fn bitmap_of(cells: &[Cell; N]) -> u128 {
    let mut b = 0u128;
    for (i, c) in cells.iter().enumerate() {
        if matches!(c, Cell::I) {
            b |= 1u128 << i;
        }
    }
    b
}

fn packed_len(cells: &[Cell; N]) -> usize {
    cells.iter().map(|c| c.size()).sum()
}

fn physical_offset(cells: &[Cell; N], i: usize) -> usize {
    cells[..i].iter().map(|c| c.size()).sum()
}

fn loader_byte_offset(bitmap: u128, tick_offset: usize) -> usize {
    let mask = if tick_offset == 0 {
        0
    } else {
        (1u128 << tick_offset) - 1
    };
    let initialized = (bitmap & mask).count_ones() as usize;
    let uninitialized = tick_offset - initialized;
    initialized * 113 + uninitialized * 1
}

/// Anchor: cast on body → real tick-data bytes available = account_len - DISC - HEADER
fn real_tick_data_len(account_len: usize) -> usize {
    account_len.saturating_sub(DISC + HEADER)
}

fn encode_body(cells: &[Cell; N]) -> Vec<u8> {
    let mut body = vec![0u8; HEADER + packed_len(cells)];
    body[0..4].copy_from_slice(&START.to_le_bytes());
    body[36..52].copy_from_slice(&bitmap_of(cells).to_le_bytes());
    let mut off = HEADER;
    for (i, c) in cells.iter().enumerate() {
        let tick = match c {
            Cell::U => DynamicTick::Uninitialized,
            Cell::I => DynamicTick::from(&TickUpdate {
                initialized: true,
                liquidity_net: (i as i128) + 1,
                liquidity_gross: (i as u128) + 2,
                fee_growth_outside_a: (i as u128) + 3,
                fee_growth_outside_b: (i as u128) + 4,
                reward_growths_outside: [5, 6, 7],
            }),
        };
        let enc = tick.try_to_vec().unwrap();
        body[off..off + enc.len()].copy_from_slice(&enc);
        off += enc.len();
    }
    body
}

#[derive(Default)]
struct Stats {
    states: usize,
    /// P2a: formed 113-byte slice extends past real tick-data end
    slice_crosses: usize,
    /// Among those, tick is Initialized (would Borsh-read 113 across boundary)
    init_crosses: usize,
    /// Among those, tick is Uninitialized (Borsh reads 1; slice still crosses)
    uninit_crosses: usize,
    /// Measured: get_tick succeeded but deserialize consumed past-boundary canary (init path)
    measured_oob_reads_113: usize,
    examples_init: Vec<String>,
    examples_uninit: Vec<String>,
}

fn check_state(cells: &[Cell; N], stats: &mut Stats) {
    stats.states += 1;
    let account_len = DISC + HEADER + packed_len(cells);
    let real_td = real_tick_data_len(account_len);
    let bm = bitmap_of(cells);
    assert_eq!(real_td, packed_len(cells));

    for i in 0..N {
        let off = physical_offset(cells, i);
        assert_eq!(off, loader_byte_offset(bm, i));
        let slice_end = off + DynamicTick::INITIALIZED_LEN; // 113
        if slice_end > real_td {
            stats.slice_crosses += 1;
            match cells[i] {
                Cell::I => {
                    stats.init_crosses += 1;
                    if stats.examples_init.len() < 5 {
                        stats.examples_init.push(format!(
                            "account_len={account_len} i={i} off={off} slice_end={slice_end} real_td={real_td}"
                        ));
                    }
                }
                Cell::U => {
                    stats.uninit_crosses += 1;
                    if stats.examples_uninit.len() < 3 {
                        stats.examples_uninit.push(format!(
                            "account_len={account_len} i={i} off={off} slice_end={slice_end} real_td={real_td}"
                        ));
                    }
                }
            }
        }
    }
}

/// Attempt production get_tick when slice crosses; detect if Initialized path reads canary.
fn measure_get_tick_cross(cells: &[Cell; N], idx: usize) -> Option<bool> {
    let account_len = DISC + HEADER + packed_len(cells);
    let real_td = real_tick_data_len(account_len);
    let off = physical_offset(cells, idx);
    if off + 113 <= real_td {
        return None;
    }

    let mut backing = vec![0u8; account_len + GUARD];
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    let body = encode_body(cells);
    backing[DISC..DISC + body.len()].copy_from_slice(&body);
    // Distinct canary past boundary
    for (i, b) in backing[account_len..].iter_mut().enumerate() {
        *b = ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY);
    }

    // Production cast on body only — claimed MAX_LEN extends into canary
    let body_slice = &mut backing[DISC..account_len];
    let loader = DynamicTickArrayLoader::load(body_slice);
    let tick_index = START + idx as i32 * SPACING as i32;
    match loader.get_tick(tick_index, SPACING) {
        Ok(tick) => {
            // If Initialized and slice crossed, deserialize must have read past boundary.
            // We can't see which bytes Borsh touched directly; infer from cell type.
            Some(matches!(cells[idx], Cell::I) && tick.initialized)
        }
        Err(_) => Some(false),
    }
}

fn all_u() -> [Cell; N] {
    [Cell::U; N]
}

fn all_i() -> [Cell; N] {
    [Cell::I; N]
}

fn one_i_at(k: usize) -> [Cell; N] {
    let mut c = all_u();
    c[k] = Cell::I;
    c
}

fn alt() -> [Cell; N] {
    let mut c = all_u();
    for i in (0..N).step_by(2) {
        c[i] = Cell::I;
    }
    c
}

fn near_full() -> [Cell; N] {
    let mut c = all_i();
    c[N - 1] = Cell::U;
    c
}

fn main() {
    println!("dta_p2b — boundary-crossing slice / Initialized deserialize analysis");
    println!(
        "MIN_LEN={} MAX_LEN={} INIT={} UNINIT={}",
        DynamicTickArray::MIN_LEN,
        DynamicTickArray::MAX_LEN,
        DynamicTick::INITIALIZED_LEN,
        DynamicTick::UNINITIALIZED_LEN
    );

    let mut stats = Stats::default();

    // Exhaustive single-I positions + structural families (not 2^88)
    check_state(&all_u(), &mut stats);
    check_state(&all_i(), &mut stats);
    check_state(&alt(), &mut stats);
    check_state(&near_full(), &mut stats);
    for k in 0..N {
        check_state(&one_i_at(k), &mut stats);
    }
    // two-I pairs at ends/middle
    for (a, b) in [(0, 1), (0, 87), (43, 44), (86, 87), (10, 50)] {
        let mut c = all_u();
        c[a] = Cell::I;
        c[b] = Cell::I;
        check_state(&c, &mut stats);
    }

    println!("\n=== INVARIANT ENUMERATION ===");
    println!("states checked: {}", stats.states);
    println!(
        "P2a slice crosses real tick-data end: {} (uninit={}, init={})",
        stats.slice_crosses, stats.uninit_crosses, stats.init_crosses
    );
    if stats.init_crosses == 0 {
        println!(
            "P2b Initialized 113-byte OOB READ: NONE under invariant-holding packed layouts."
        );
        println!(
            "Reason: every I record lies fully inside packed_len; off+113 <= real_td always."
        );
    } else {
        println!("P2b CANDIDATES (Initialized crosses):");
        for e in &stats.examples_init {
            println!("  {e}");
        }
    }
    println!("P2a Uninitialized slice-cross examples (Borsh reads 1 byte only):");
    for e in &stats.examples_uninit {
        println!("  {e}");
    }

    // Measure get_tick on crossing Uninitialized (should succeed, 1-byte read)
    println!("\n=== MEASURE get_tick on crossing Uninitialized (empty account, last ticks) ===");
    let empty = all_u();
    for idx in [0usize, 1, 50, 87] {
        let off = physical_offset(&empty, idx);
        let real_td = 88;
        let crosses = off + 113 > real_td;
        let r = measure_get_tick_cross(&empty, idx);
        println!("  empty idx={idx} off={off} crosses={crosses} get_tick_ok_init_path={r:?}");
    }

    // Measure get_tick on Initialized last tick of full account (should NOT cross)
    println!("\n=== MEASURE get_tick on Initialized at full / near-full (should not cross) ===");
    for (label, cells, idx) in [
        ("all_i last", all_i(), N - 1),
        ("one_i at 87", one_i_at(87), 87),
        ("near_full last U — get idx 0 I", near_full(), 0),
    ] {
        let account_len = DISC + HEADER + packed_len(&cells);
        let off = physical_offset(&cells, idx);
        let real_td = real_tick_data_len(account_len);
        println!(
            "  {label}: account_len={account_len} off={off} off+113={} real_td={real_td} crosses={}",
            off + 113,
            off + 113 > real_td
        );
        if let Some(v) = measure_get_tick_cross(&cells, idx) {
            if v {
                stats.measured_oob_reads_113 += 1;
            }
            println!("    measured init OOB read marker={v}");
        } else {
            println!("    no cross — skip measure");
        }
    }

    // Anchor +8: full MAX account body is still 8 short of MAX_LEN claim
    println!("\n=== ANCHOR +8 on full account ===");
    let full_account = DynamicTickArray::MAX_LEN; // 10004
    let body_len = full_account - DISC; // 9996
    let real_td = body_len - HEADER; // 9944
    // Loader is [u8; MAX_LEN] cast on body ⇒ claims 10004 body bytes; real body 9996; +8.
    // tick_data = loader[52..] claims 9952; real tick bytes = 9944; envelope in tick_data = 8.
    println!(
        "  full account_len={full_account} body={body_len} real_td={real_td} claimed_td_via_body_cast=9952 envelope=8"
    );
    let all = all_i();
    assert_eq!(packed_len(&all), 9944);
    let off_last = physical_offset(&all, N - 1);
    println!(
        "  last I: off={off_last} off+113={} vs real_td={real_td} crosses={}",
        off_last + 113,
        off_last + 113 > real_td
    );

    println!("\n=== P2b VERDICT ===");
    if stats.init_crosses == 0 && stats.measured_oob_reads_113 == 0 {
        println!(
            "Under reachable invariant-holding layouts: no Initialized 113-byte physical OOB READ."
        );
        println!(
            "Uninit ticks CAN form P2a slices past the real end; deserialize reads 1 in-bounds byte."
        );
        println!(
            "P2b reopen conditions: broken bitmap↔layout invariant, or mid-update inconsistent view."
        );
    } else {
        println!(
            "INITIALIZED OOB READ CANDIDATES FOUND: init_crosses={} measured={}",
            stats.init_crosses, stats.measured_oob_reads_113
        );
        std::process::exit(1);
    }
}
