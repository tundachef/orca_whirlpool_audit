//! Phase D — shared DynamicTickArray attacker/victim host experiment.
//!
//! Question:
//!   After an attacker-triggered Uninit↔Init transition on a shared TA
//!   (production `update_tick` on short account + canary), does the victim’s
//!   subsequent program-visible state diverge from a control that applies the
//!   *same logical transition* without the short-account OOB path?
//!
//! Arms:
//!   CONTROL      — transition via production loader on MAX_LEN backing (no short OOB)
//!   EXPERIMENTAL — transition via production loader on exact-sized account + canary
//!   BASELINE     — no attacker transition (victim sees pre-attacker state)
//!
//! Victim observables (host stand-ins for later LiteSVM):
//!   - get_tick at victim lower/upper
//!   - next_fee_growths_inside
//!   - signed liquidity_net applied as if swap crossed the tick
//!   - raw committed account body bytes

use borsh::BorshSerialize;
use whirlpool::manager::tick_manager::next_fee_growths_inside;
use whirlpool::math::add_liquidity_delta;
use whirlpool::state::{
    DynamicTick, DynamicTickArray, DynamicTickArrayLoader, Tick, TickArrayType, TickUpdate,
    TICK_ARRAY_SIZE_USIZE,
};

const DISC: usize = 8;
const HEADER: usize = 4 + 32 + 16;
const TICK_DATA_OFF: usize = HEADER;
const START_TICK_INDEX: i32 = 0;
const TICK_SPACING: u16 = 1;
const N: usize = TICK_ARRAY_SIZE_USIZE;
const GUARD: usize = 16_384;
const CANARY: u8 = 0xA5;

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

fn sample_init(seed: u64) -> TickUpdate {
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

#[derive(Clone, Debug)]
struct RefState {
    cells: [Cell; N],
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

    fn set_init(&mut self, i: usize, seed: u64) {
        self.cells[i] = Cell::I;
        self.seeds[i] = seed;
    }

    fn set_uninit(&mut self, i: usize) {
        self.cells[i] = Cell::U;
        self.seeds[i] = 0;
    }

    fn encode_body(&self) -> Vec<u8> {
        let mut body = vec![0u8; HEADER + self.packed_len()];
        body[0..4].copy_from_slice(&START_TICK_INDEX.to_le_bytes());
        body[36..52].copy_from_slice(&self.bitmap().to_le_bytes());
        let mut off = TICK_DATA_OFF;
        for i in 0..N {
            let tick = match self.cells[i] {
                Cell::U => DynamicTick::Uninitialized,
                Cell::I => DynamicTick::from(&sample_init(self.seeds[i])),
            };
            let enc = tick.try_to_vec().unwrap();
            body[off..off + enc.len()].copy_from_slice(&enc);
            off += enc.len();
        }
        body
    }
}

fn paint_canary(buf: &mut [u8], account_len: usize) {
    for (i, b) in buf[account_len..].iter_mut().enumerate() {
        *b = ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY);
    }
}

fn tick_index(offset: usize) -> i32 {
    START_TICK_INDEX + offset as i32 * TICK_SPACING as i32
}

/// Apply transition on MAX_LEN backing (control — no short-account OOB).
fn apply_control(before: &RefState, idx: usize, init: bool, seed: u64) -> Vec<u8> {
    let mut body = [0u8; DynamicTickArray::MAX_LEN];
    let enc = before.encode_body();
    body[..enc.len()].copy_from_slice(&enc);
    let loader = DynamicTickArrayLoader::load_mut(&mut body);
    let update = if init {
        sample_init(seed)
    } else {
        TickUpdate::default()
    };
    loader
        .update_tick(tick_index(idx), TICK_SPACING, &update)
        .expect("control update_tick");
    // Return committed logical body only
    let mut after = before.clone();
    if init {
        after.set_init(idx, seed);
    } else {
        after.set_uninit(idx);
    }
    body[..after.encode_body().len()].to_vec()
}

/// Apply transition on exact-sized account + canary (experimental — Phase A OOB path).
fn apply_experimental(before: &RefState, idx: usize, init: bool, seed: u64) -> Vec<u8> {
    let mut after = before.clone();
    if init {
        after.set_init(idx, seed);
    } else {
        after.set_uninit(idx);
    }
    let n_at_rotate = if init {
        after.account_len()
    } else {
        before.account_len()
    };
    let n_committed = after.account_len();

    let mut backing = vec![0u8; n_at_rotate + GUARD];
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    let before_body = before.encode_body();
    backing[DISC..DISC + before_body.len()].copy_from_slice(&before_body);
    paint_canary(&mut backing, n_at_rotate);

    {
        let body = &mut backing[DISC..n_at_rotate];
        let loader = DynamicTickArrayLoader::load_mut(body);
        let update = if init {
            sample_init(seed)
        } else {
            TickUpdate::default()
        };
        loader
            .update_tick(tick_index(idx), TICK_SPACING, &update)
            .expect("experimental update_tick");
    }

    backing[DISC..n_committed].to_vec()
}

#[derive(Debug, Clone, PartialEq)]
struct VictimView {
    lower: Tick,
    upper: Tick,
    fee_inside: (u128, u128),
    /// Pool liquidity after hypothetically crossing lower a_to_b (uses -liquidity_net).
    liquidity_after_cross_lower_a_to_b: Result<u128, String>,
    body: Vec<u8>,
}

fn victim_observe(body: &[u8], lower_off: usize, upper_off: usize) -> VictimView {
    // Load via production loader on a MAX_LEN pad of the committed body
    // (victim read path after account is committed at exact size — pad only for cast).
    let mut padded = vec![0u8; DynamicTickArray::MAX_LEN];
    padded[..body.len()].copy_from_slice(body);
    let loader = DynamicTickArrayLoader::load(&padded);

    let lower = loader
        .get_tick(tick_index(lower_off), TICK_SPACING)
        .expect("victim get_tick lower");
    let upper = loader
        .get_tick(tick_index(upper_off), TICK_SPACING)
        .expect("victim get_tick upper");

    // Synthetic pool state for fee-inside
    let tick_current = tick_index(lower_off); // price at lower
    let fee_global_a = 10_000u128 << 64;
    let fee_global_b = 20_000u128 << 64;
    let fee_inside = next_fee_growths_inside(
        tick_current,
        &lower,
        tick_index(lower_off),
        &upper,
        tick_index(upper_off),
        fee_global_a,
        fee_global_b,
    );

    let pool_liq = 1_000_000u128;
    let signed = -lower.liquidity_net; // a_to_b cross
    let liquidity_after_cross_lower_a_to_b =
        add_liquidity_delta(pool_liq, signed).map_err(|e| format!("{e:?}"));

    VictimView {
        lower,
        upper,
        fee_inside,
        liquidity_after_cross_lower_a_to_b,
        body: body.to_vec(),
    }
}

fn cmp_views(label: &str, a: &VictimView, b: &VictimView) -> Vec<String> {
    let mut diffs = Vec::new();
    if a.lower != b.lower {
        diffs.push(format!("{label}: lower tick diverge"));
    }
    if a.upper != b.upper {
        diffs.push(format!("{label}: upper tick diverge"));
    }
    if a.fee_inside != b.fee_inside {
        diffs.push(format!(
            "{label}: fee_inside diverge {:?} vs {:?}",
            a.fee_inside, b.fee_inside
        ));
    }
    if a.liquidity_after_cross_lower_a_to_b != b.liquidity_after_cross_lower_a_to_b {
        diffs.push(format!(
            "{label}: liquidity_after_cross diverge {:?} vs {:?}",
            a.liquidity_after_cross_lower_a_to_b, b.liquidity_after_cross_lower_a_to_b
        ));
    }
    if a.body != b.body {
        let first = a
            .body
            .iter()
            .zip(b.body.iter())
            .position(|(x, y)| x != y);
        diffs.push(format!("{label}: committed body diverge first={first:?}"));
    }
    diffs
}

struct Scenario {
    name: &'static str,
    setup: RefState,
    /// Attacker transition
    atk_idx: usize,
    atk_init: bool,
    atk_seed: u64,
    /// Victim position ticks (offsets in array)
    vic_lower: usize,
    vic_upper: usize,
}

fn run_scenario(s: &Scenario) -> usize {
    println!("\n=== SCENARIO: {} ===", s.name);
    println!(
        "  setup account_len={} bitmap={:#x}",
        s.setup.account_len(),
        s.setup.bitmap()
    );
    println!(
        "  attacker: idx={} init={}  victim: lower={} upper={}",
        s.atk_idx, s.atk_init, s.vic_lower, s.vic_upper
    );

    let baseline_body = s.setup.encode_body();
    let control_body = apply_control(&s.setup, s.atk_idx, s.atk_init, s.atk_seed);
    let experimental_body = apply_experimental(&s.setup, s.atk_idx, s.atk_init, s.atk_seed);

    let baseline = victim_observe(&baseline_body, s.vic_lower, s.vic_upper);
    let control = victim_observe(&control_body, s.vic_lower, s.vic_upper);
    let experimental = victim_observe(&experimental_body, s.vic_lower, s.vic_upper);

    // Attacker should change shared state vs baseline (when transition is real)
    let vs_base = cmp_views("experimental_vs_baseline", &experimental, &baseline);
    let changed = !vs_base.is_empty();
    println!(
        "  attacker changed victim-visible state vs baseline: {changed} ({})",
        if changed {
            "expected for real transition"
        } else {
            "no-op or victim ticks unaffected"
        }
    );

    // Critical comparison: control (correct transition) vs experimental (OOB path)
    let diffs = cmp_views("control_vs_experimental", &control, &experimental);
    if diffs.is_empty() {
        println!("  CONTROL ≡ EXPERIMENTAL — OOB path did not alter victim observables");
    } else {
        for d in &diffs {
            println!("  DIFF {d}");
        }
    }

    // Print a sample economic snapshot from experimental (copy packed fields first)
    let lower_init = experimental.lower.initialized;
    let lower_net = experimental.lower.liquidity_net;
    let lower_fee_a = experimental.lower.fee_growth_outside_a;
    println!(
        "  victim.lower.initialized={lower_init} liquidity_net={lower_net} fee_out_a={lower_fee_a}"
    );
    println!(
        "  fee_inside={:?} liq_after_cross_lower={:?}",
        experimental.fee_inside, experimental.liquidity_after_cross_lower_a_to_b
    );

    diffs.len()
}

fn half_setup() -> RefState {
    let mut s = RefState::empty();
    for i in (0..N).step_by(2) {
        s.set_init(i, 50 + i as u64);
    }
    s
}

fn main() {
    println!("dta_shared_ta Phase D — shared TA control vs experimental (host)");
    println!(
        "MIN_LEN={} MAX_LEN={} N={N}",
        DynamicTickArray::MIN_LEN,
        DynamicTickArray::MAX_LEN
    );

    let mut total_diffs = 0usize;

    // 1) Attacker inits a new tick on empty TA; victim reads that tick + another
    {
        let s = Scenario {
            name: "empty→init tick7; victim uses 7 and 20",
            setup: RefState::empty(),
            atk_idx: 7,
            atk_init: true,
            atk_seed: 777,
            vic_lower: 7,
            vic_upper: 20,
        };
        total_diffs += run_scenario(&s);
    }

    // 2) Shared half-populated; attacker inits interior U; victim straddles it
    {
        let s = Scenario {
            name: "half-pop; attacker inits 15; victim 10..30",
            setup: half_setup(),
            atk_idx: 15,
            atk_init: true,
            atk_seed: 1515,
            vic_lower: 10,
            vic_upper: 30,
        };
        total_diffs += run_scenario(&s);
    }

    // 3) Attacker de-inits a tick the victim uses as lower
    {
        let setup = half_setup();
        // ensure 10 is initialized (even idxs are)
        assert_eq!(setup.cells[10], Cell::I);
        let s = Scenario {
            name: "half-pop; attacker uninits victim lower=10",
            setup,
            atk_idx: 10,
            atk_init: false,
            atk_seed: 0,
            vic_lower: 10,
            vic_upper: 30,
        };
        total_diffs += run_scenario(&s);
    }

    // 4) Near-full; attacker inits last tick; victim far away
    {
        let mut setup = RefState::empty();
        for i in 0..N - 1 {
            setup.set_init(i, 2000 + i as u64);
        }
        let s = Scenario {
            name: "near-full; attacker inits last; victim 0..10",
            setup,
            atk_idx: N - 1,
            atk_init: true,
            atk_seed: 8888,
            vic_lower: 0,
            vic_upper: 10,
        };
        total_diffs += run_scenario(&s);
    }

    // 5) Near-full; attacker uninits mid tick victim uses
    {
        let mut setup = RefState::empty();
        for i in 0..N {
            setup.set_init(i, 3000 + i as u64);
        }
        let s = Scenario {
            name: "full; attacker uninits mid=44; victim 40..50",
            setup,
            atk_idx: 44,
            atk_init: false,
            atk_seed: 0,
            vic_lower: 40,
            vic_upper: 50,
        };
        total_diffs += run_scenario(&s);
    }

    // 6) Attacker transition on tick victim does NOT use — still compare bodies
    {
        let s = Scenario {
            name: "half-pop; attacker inits 87; victim 2..8 (disjoint)",
            setup: half_setup(),
            atk_idx: 87,
            atk_init: true,
            atk_seed: 87,
            vic_lower: 2,
            vic_upper: 8,
        };
        total_diffs += run_scenario(&s);
    }

    println!("\n=== PHASE D SUMMARY ===");
    if total_diffs == 0 {
        println!(
            "All scenarios: CONTROL ≡ EXPERIMENTAL on victim observables \
             (get_tick, fee_inside, cross liquidity, committed body)."
        );
        println!(
            "Attacker can change shared TA state (vs baseline), but the short-account OOB path \
             does not produce additional victim-visible divergence beyond the correct transition."
        );
        println!("Next: persistence/atomicity under SVM; P2b boundary deserialize; Phase E envelope.");
    } else {
        println!("TOTAL control↔experimental DIFFS: {total_diffs}");
        std::process::exit(1);
    }
}
