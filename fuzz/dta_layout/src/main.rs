//! Phase B — bitmap ↔ packed-layout differential + bounded reference oracle.
//!
//! REFERENCE: exact-sized packed buffer; offsets from bitmap; no MAX_LEN cast.
//! VULNERABLE: production `DynamicTickArrayLoader::update_tick` on a MAX_LEN host buffer
//!            (physical allocation = claimed size, so this track isolates *semantic*
//!            divergence from OOB — OOB is Phase A).
//!
//! After every transition verify:
//!   bitmap == packed init tags
//!   computed_byte_offset(i) == actual serialized offset(i)
//!   decode(encode(state)) == state
//!   reference account body (within real len) == vulnerable body prefix

use borsh::{BorshDeserialize, BorshSerialize};
use whirlpool::state::{
    DynamicTick, DynamicTickArray, DynamicTickArrayLoader, TickArrayType, TickUpdate,
    TICK_ARRAY_SIZE_USIZE,
};

const DISC: usize = 8;
const HEADER: usize = 4 + 32 + 16; // start + whirlpool + bitmap
const TICK_DATA_OFF: usize = HEADER; // 52 in body coords
const START_TICK_INDEX: i32 = 0;
const TICK_SPACING: u16 = 1;
const N: usize = TICK_ARRAY_SIZE_USIZE; // 88

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    U,
    I,
}

impl Cell {
    fn size(self) -> usize {
        match self {
            Cell::U => DynamicTick::UNINITIALIZED_LEN,
            Cell::I => DynamicTick::INITIALIZED_LEN,
        }
    }
}

fn sample_init_update(seed: u64) -> TickUpdate {
    TickUpdate {
        initialized: true,
        liquidity_net: (seed as i128).wrapping_mul(17) + 1,
        liquidity_gross: seed.wrapping_mul(19).wrapping_add(3) as u128,
        fee_growth_outside_a: seed.wrapping_mul(23) as u128,
        fee_growth_outside_b: seed.wrapping_mul(29) as u128,
        reward_growths_outside: [
            seed.wrapping_mul(31) as u128,
            seed.wrapping_mul(37) as u128,
            seed.wrapping_mul(41) as u128,
        ],
    }
}

fn encode_cell(cell: Cell, seed: u64) -> Vec<u8> {
    match cell {
        Cell::U => {
            let t = DynamicTick::Uninitialized;
            t.try_to_vec().unwrap()
        }
        Cell::I => {
            let u = sample_init_update(seed);
            DynamicTick::from(&u).try_to_vec().unwrap()
        }
    }
}

/// Bounded reference: owns exact packed body (no discriminator).
#[derive(Clone, Debug)]
struct RefState {
    cells: [Cell; N],
    /// Per-tick seeds for Initialized payloads (ignored when U).
    seeds: [u64; N],
}

impl RefState {
    fn empty() -> Self {
        Self {
            cells: [Cell::U; N],
            seeds: [0; N],
        }
    }

    fn bitmap(&self) -> u128 {
        let mut b = 0u128;
        for (i, c) in self.cells.iter().enumerate() {
            if *c == Cell::I {
                b |= 1u128 << i;
            }
        }
        b
    }

    fn packed_len(&self) -> usize {
        self.cells.iter().map(|c| c.size()).sum()
    }

    fn account_len(&self) -> usize {
        DISC + HEADER + self.packed_len()
    }

    fn physical_offset(&self, i: usize) -> usize {
        self.cells[..i].iter().map(|c| c.size()).sum()
    }

    /// Loader-style byte_offset from bitmap alone (must match physical_offset).
    fn loader_byte_offset(&self, tick_offset: usize) -> usize {
        let bitmap = self.bitmap();
        let mask = if tick_offset == 0 {
            0
        } else {
            (1u128 << tick_offset) - 1
        };
        let initialized = (bitmap & mask).count_ones() as usize;
        let uninitialized = tick_offset - initialized;
        initialized * DynamicTick::INITIALIZED_LEN + uninitialized * DynamicTick::UNINITIALIZED_LEN
    }

    fn encode_body(&self) -> Vec<u8> {
        let mut body = vec![0u8; HEADER + self.packed_len()];
        body[0..4].copy_from_slice(&START_TICK_INDEX.to_le_bytes());
        // whirlpool zeros
        body[36..52].copy_from_slice(&self.bitmap().to_le_bytes());
        let mut off = TICK_DATA_OFF;
        for i in 0..N {
            let enc = encode_cell(self.cells[i], self.seeds[i]);
            body[off..off + enc.len()].copy_from_slice(&enc);
            off += enc.len();
        }
        assert_eq!(off, body.len());
        body
    }

    fn set_init(&mut self, i: usize, seed: u64) {
        self.cells[i] = Cell::I;
        self.seeds[i] = seed;
    }

    fn set_uninit(&mut self, i: usize) {
        self.cells[i] = Cell::U;
        self.seeds[i] = 0;
    }

    fn check_invariants(&self, label: &str) -> Result<(), String> {
        for i in 0..N {
            let phys = self.physical_offset(i);
            let loader = self.loader_byte_offset(i);
            if phys != loader {
                return Err(format!(
                    "{label}: offset mismatch at i={i}: physical={phys} loader_byte_offset={loader}"
                ));
            }
        }
        // decode(encode) roundtrip on packed ticks
        let body = self.encode_body();
        let ticks = &body[TICK_DATA_OFF..];
        let mut off = 0usize;
        for i in 0..N {
            let want = self.cells[i].size();
            if off + want > ticks.len() {
                return Err(format!("{label}: packed truncated at i={i}"));
            }
            let mut slice = &ticks[off..off + want];
            // For I, deserialize needs full 113; for U, 1 byte is enough but we pass exact size.
            // Borsh enum: read tag from first byte.
            let tag = ticks[off];
            match self.cells[i] {
                Cell::U => {
                    if tag != 0 {
                        return Err(format!("{label}: cell {i} expected U tag0 got {tag}"));
                    }
                }
                Cell::I => {
                    if tag != 1 {
                        return Err(format!("{label}: cell {i} expected I tag1 got {tag}"));
                    }
                    // full deserialize
                    let mut full = &ticks[off..off + DynamicTick::INITIALIZED_LEN];
                    let decoded = DynamicTick::deserialize(&mut full)
                        .map_err(|e| format!("{label}: decode I at {i}: {e}"))?;
                    let expected = DynamicTick::from(&sample_init_update(self.seeds[i]));
                    if decoded != expected {
                        return Err(format!("{label}: payload mismatch at i={i}"));
                    }
                }
            }
            let _ = &mut slice; // silence
            off += want;
        }
        if off != ticks.len() {
            return Err(format!(
                "{label}: trailing packed bytes: off={off} len={}",
                ticks.len()
            ));
        }
        let expect_account = DISC + HEADER + self.packed_len();
        if self.account_len() != expect_account {
            return Err(format!("{label}: account_len inconsistency"));
        }
        // MIN/MAX bounds
        if self.account_len() < DynamicTickArray::MIN_LEN
            || self.account_len() > DynamicTickArray::MAX_LEN
        {
            return Err(format!(
                "{label}: account_len {} outside [{},{}]",
                self.account_len(),
                DynamicTickArray::MIN_LEN,
                DynamicTickArray::MAX_LEN
            ));
        }
        Ok(())
    }
}

/// Apply the same transition through production loader on a MAX_LEN buffer.
fn vuln_apply(body_max: &mut [u8; DynamicTickArray::MAX_LEN], idx: usize, init: bool, seed: u64) {
    let loader = DynamicTickArrayLoader::load_mut(body_max);
    let tick_index = START_TICK_INDEX + idx as i32 * TICK_SPACING as i32;
    let update = if init {
        sample_init_update(seed)
    } else {
        TickUpdate::default()
    };
    loader
        .update_tick(tick_index, TICK_SPACING, &update)
        .unwrap_or_else(|e| panic!("update_tick failed idx={idx} init={init}: {e:?}"));
}

fn body_prefix_from_ref(r: &RefState) -> Vec<u8> {
    r.encode_body()
}

fn init_vuln_empty() -> [u8; DynamicTickArray::MAX_LEN] {
    let mut body = [0u8; DynamicTickArray::MAX_LEN];
    body[0..4].copy_from_slice(&START_TICK_INDEX.to_le_bytes());
    body
}

/// Compare reference packed body to vulnerable MAX_LEN body prefix of the same logical length.
fn diff_bodies(label: &str, r: &RefState, vuln: &[u8]) -> Result<(), String> {
    let expected = body_prefix_from_ref(r);
    let real_body_len = expected.len();
    if vuln.len() < real_body_len {
        return Err(format!("{label}: vuln shorter than ref body"));
    }
    if vuln[..real_body_len] != expected[..] {
        // find first mismatch
        for i in 0..real_body_len {
            if vuln[i] != expected[i] {
                return Err(format!(
                    "{label}: body diverge at byte {i}: vuln={:#x} ref={:#x} (real_body_len={real_body_len})",
                    vuln[i], expected[i]
                ));
            }
        }
    }
    Ok(())
}

struct Stats {
    transitions: usize,
    invariant_ok: usize,
    diffs: Vec<String>,
}

fn run_sequence(name: &str, steps: &[(usize, bool /*init*/)]) -> Stats {
    println!("\n=== sequence: {name} ({} steps) ===", steps.len());
    let mut r = RefState::empty();
    let mut vuln = init_vuln_empty();
    let mut stats = Stats {
        transitions: 0,
        invariant_ok: 0,
        diffs: vec![],
    };

    // Initial agree
    if let Err(e) = r.check_invariants("init") {
        stats.diffs.push(e);
    } else {
        stats.invariant_ok += 1;
    }
    if let Err(e) = diff_bodies("init", &r, &vuln) {
        stats.diffs.push(e);
    }

    for (step, &(idx, init)) in steps.iter().enumerate() {
        let seed = 1000 + step as u64 * 7 + idx as u64;
        // Skip no-ops that would not change size (already in desired state)
        let already = r.cells[idx] == if init { Cell::I } else { Cell::U };
        if already {
            continue;
        }

        if init {
            r.set_init(idx, seed);
        } else {
            r.set_uninit(idx);
        }
        vuln_apply(&mut vuln, idx, init, seed);
        stats.transitions += 1;

        let label = format!("{name}#{step} idx={idx} init={init}");
        match r.check_invariants(&label) {
            Ok(()) => stats.invariant_ok += 1,
            Err(e) => stats.diffs.push(e),
        }
        if let Err(e) = diff_bodies(&label, &r, &vuln) {
            stats.diffs.push(e);
        }
    }

    println!(
        "  transitions={} invariant_checks_ok={} diffs={}",
        stats.transitions,
        stats.invariant_ok,
        stats.diffs.len()
    );
    for d in stats.diffs.iter().take(8) {
        println!("  DIFF: {d}");
    }
    if stats.diffs.len() > 8 {
        println!("  ... {} more", stats.diffs.len() - 8);
    }
    stats
}

fn seq_ascending_init() -> Vec<(usize, bool)> {
    (0..N).map(|i| (i, true)).collect()
}

fn seq_ascending_then_clear() -> Vec<(usize, bool)> {
    let mut s = seq_ascending_init();
    s.extend((0..N).rev().map(|i| (i, false)));
    s
}

fn seq_pingpong() -> Vec<(usize, bool)> {
    let mut s = Vec::new();
    for k in 0..N / 2 {
        s.push((k, true));
        s.push((N - 1 - k, true));
    }
    if N % 2 == 1 {
        s.push((N / 2, true));
    }
    for k in 0..N / 2 {
        s.push((k, false));
        s.push((N - 1 - k, false));
    }
    if N % 2 == 1 {
        s.push((N / 2, false));
    }
    s
}

/// Interleave pattern: I U I U ... then scramble by clearing even then odd.
fn seq_alternate_then_scramble() -> Vec<(usize, bool)> {
    let mut s: Vec<(usize, bool)> = (0..N).map(|i| (i, i % 2 == 0)).collect();
    // clear all even (the I ones)
    for i in (0..N).step_by(2) {
        s.push((i, false));
    }
    // init all odd
    for i in (1..N).step_by(2) {
        s.push((i, true));
    }
    // clear all
    for i in 0..N {
        s.push((i, false));
    }
    s
}

/// Randomish LCG walk over indices with mix of init/uninit.
fn seq_lcg(steps: usize, seed: u64) -> Vec<(usize, bool)> {
    let mut s = Vec::with_capacity(steps);
    let mut x = seed;
    for _ in 0..steps {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let idx = (x as usize) % N;
        let init = ((x >> 17) & 1) == 1;
        s.push((idx, init));
    }
    s
}

const GUARD: usize = 16_384;
const CANARY: u8 = 0xA5;

fn paint_canary(buf: &mut [u8], account_len: usize) {
    for (i, b) in buf[account_len..].iter_mut().enumerate() {
        *b = ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY);
    }
}

/// Short-account pollution track:
/// Run production `update_tick` on an exact-sized account whose past-boundary
/// bytes are gradient canary. After the transition, compare the **committed
/// logical body** (post-resize length) to the bounded reference.
///
/// If they diverge, OOB bytes were rotated into program-visible packed state.
fn pollution_transition(
    label: &str,
    before: &RefState,
    idx: usize,
    init: bool,
    seed: u64,
) -> Result<(), String> {
    let mut after = before.clone();
    if init {
        if after.cells[idx] == Cell::I {
            return Ok(());
        }
        after.set_init(idx, seed);
    } else {
        if after.cells[idx] == Cell::U {
            return Ok(());
        }
        after.set_uninit(idx);
    }

    // Production resize order
    let n_at_rotate = if init {
        // increase: resize first to AFTER size, then rotate
        after.account_len()
    } else {
        // decrease: rotate at BEFORE size, then shrink
        before.account_len()
    };
    let n_committed = after.account_len();

    let mut backing = vec![0u8; n_at_rotate + GUARD];
    // disc
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    let before_body = before.encode_body();
    // For increase, account grows: copy before body then zero-fill the growth.
    // For decrease, account still large: before body already fills n_at_rotate - DISC.
    let body_at_rotate = n_at_rotate - DISC;
    if before_body.len() > body_at_rotate {
        return Err(format!("{label}: before body longer than rotate account"));
    }
    backing[DISC..DISC + before_body.len()].copy_from_slice(&before_body);
    // growth region (increase) already zero
    paint_canary(&mut backing, n_at_rotate);

    {
        let body = &mut backing[DISC..n_at_rotate];
        let loader = DynamicTickArrayLoader::load_mut(body);
        let tick_index = START_TICK_INDEX + idx as i32 * TICK_SPACING as i32;
        let update = if init {
            sample_init_update(seed)
        } else {
            TickUpdate::default()
        };
        loader
            .update_tick(tick_index, TICK_SPACING, &update)
            .map_err(|e| format!("{label}: update_tick: {e:?}"))?;
    }

    let expected = after.encode_body();
    assert_eq!(expected.len(), n_committed - DISC);
    let got = &backing[DISC..n_committed];
    if got != expected.as_slice() {
        let mut first = None;
        for i in 0..expected.len() {
            if got[i] != expected[i] {
                first = Some(i);
                break;
            }
        }
        return Err(format!(
            "{label}: LOGICAL BODY DIVERGED after OOB rotate (first_mismatch={first:?}, \
             n_at_rotate={n_at_rotate}, n_committed={n_committed}). \
             Past-boundary bytes appear to have polluted program-visible packed state."
        ));
    }
    Ok(())
}

fn run_pollution_suite() -> usize {
    println!("\n=== POLLUTION TRACK: short account + canary → logical body vs reference ===");
    let mut diffs = 0usize;

    // Case 1: empty → first tick at 0 (classic first-tick increase)
    {
        let before = RefState::empty();
        match pollution_transition("inc_first_tick0", &before, 0, true, 42) {
            Ok(()) => println!("  OK  inc_first_tick0 — no logical pollution"),
            Err(e) => {
                println!("  DIFF {e}");
                diffs += 1;
            }
        }
    }

    // Case 2: empty → init tick at various offsets (resize to MIN+112, only one I)
    for idx in [0usize, 1, 7, 44, 87] {
        let before = RefState::empty();
        let label = format!("inc_empty_to_tick{idx}");
        match pollution_transition(&label, &before, idx, true, 100 + idx as u64) {
            Ok(()) => println!("  OK  {label}"),
            Err(e) => {
                println!("  DIFF {e}");
                diffs += 1;
            }
        }
    }

    // Case 3: one tick → empty (decrease last tick)
    for idx in [0usize, 1, 7, 44, 87] {
        let mut before = RefState::empty();
        before.set_init(idx, 7);
        let label = format!("dec_tick{idx}_to_empty");
        match pollution_transition(&label, &before, idx, false, 0) {
            Ok(()) => println!("  OK  {label}"),
            Err(e) => {
                println!("  DIFF {e}");
                diffs += 1;
            }
        }
    }

    // Case 4: half-populated, init/uninit interior ticks
    let mut half = RefState::empty();
    for i in (0..N).step_by(2) {
        half.set_init(i, 50 + i as u64);
    }
    for idx in [1usize, 3, 15, 43, 87] {
        // init an uninitialized interior tick
        let label = format!("inc_half_tick{idx}");
        match pollution_transition(&label, &half, idx, true, 900 + idx as u64) {
            Ok(()) => println!("  OK  {label}"),
            Err(e) => {
                println!("  DIFF {e}");
                diffs += 1;
            }
        }
    }
    for idx in [0usize, 10, 40, 86] {
        let label = format!("dec_half_tick{idx}");
        match pollution_transition(&label, &half, idx, false, 0) {
            Ok(()) => println!("  OK  {label}"),
            Err(e) => {
                println!("  DIFF {e}");
                diffs += 1;
            }
        }
    }

    // Case 5: near-full Anchor-sized — start from 87 inits, init last / uninit one
    let mut near = RefState::empty();
    for i in 0..N - 1 {
        near.set_init(i, 2000 + i as u64);
    }
    match pollution_transition("inc_near_full_last", &near, N - 1, true, 3000) {
        Ok(()) => println!("  OK  inc_near_full_last"),
        Err(e) => {
            println!("  DIFF {e}");
            diffs += 1;
        }
    }
    let mut full = near.clone();
    full.set_init(N - 1, 3000);
    match pollution_transition("dec_full_first", &full, 0, false, 0) {
        Ok(()) => println!("  OK  dec_full_first"),
        Err(e) => {
            println!("  DIFF {e}");
            diffs += 1;
        }
    }
    match pollution_transition("dec_full_mid", &full, 44, false, 0) {
        Ok(()) => println!("  OK  dec_full_mid"),
        Err(e) => {
            println!("  DIFF {e}");
            diffs += 1;
        }
    }
    match pollution_transition("dec_full_last", &full, N - 1, false, 0) {
        Ok(()) => println!("  OK  dec_full_last"),
        Err(e) => {
            println!("  DIFF {e}");
            diffs += 1;
        }
    }

    println!("  pollution diffs: {diffs}");
    diffs
}

fn main() {
    println!("dta_layout Phase B — bitmap↔packed differential + bounded reference");
    println!(
        "N={N} MIN_LEN={} MAX_LEN={} U={} I={}",
        DynamicTickArray::MIN_LEN,
        DynamicTickArray::MAX_LEN,
        DynamicTick::UNINITIALIZED_LEN,
        DynamicTick::INITIALIZED_LEN,
    );

    let mut all_diffs = 0usize;
    for (name, steps) in [
        ("asc_init_all", seq_ascending_init()),
        ("asc_init_then_clear", seq_ascending_then_clear()),
        ("pingpong", seq_pingpong()),
        ("alternate_scramble", seq_alternate_then_scramble()),
        ("lcg_2k", seq_lcg(2000, 0xD7A1_A407)),
        ("lcg_2k_b", seq_lcg(2000, 0xBADC_0FFE)),
    ] {
        let st = run_sequence(name, &steps);
        all_diffs += st.diffs.len();
    }

    let pollution_diffs = run_pollution_suite();
    all_diffs += pollution_diffs;

    println!("\n=== SUMMARY ===");
    if all_diffs == 0 {
        println!("Semantic track: reference invariants held; reference ≡ production on MAX_LEN backing.");
        println!("Pollution track: no logical-body divergence vs reference under tested short-account transitions.");
        println!("OOB writes are real (Phase A), but under these cases rotate+serialize+resize left committed packed state intact.");
        println!("Next: expand pollution cases; Phase C data-flow; Phase D shared-TA.");
    } else {
        println!("TOTAL DIFFS: {all_diffs}");
        println!("Divergence FOUND — investigate before Phase C/D.");
        std::process::exit(1);
    }
}
