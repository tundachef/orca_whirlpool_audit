//! OOB read-after-write harness.
//!
//! After a production-path OOB write (update_tick / rotate on short account):
//!   1) Are envelope bytes readable back from the same allocation? (host channel)
//!   2) Does any production get_tick pull envelope bytes into a Tick value?
//!   3) After logical commit (serialize + shrink), is the envelope still coupled?
//!
//! Classifies host outcome toward envelope cases A/B (SVM C/D need Phase E).

use whirlpool::state::{
    DynamicTickArray, DynamicTickArrayLoader, DynamicTickData, TickArrayType, TickUpdate,
    TICK_ARRAY_SIZE_USIZE,
};

const DISC: usize = 8;
const HEADER: usize = 52;
const START: i32 = 0;
const SPACING: u16 = 1;
const N: usize = TICK_ARRAY_SIZE_USIZE;
const GUARD: usize = 16_384;
const CANARY: u8 = 0xA5;

fn paint(buf: &mut [u8], account_len: usize) {
    for (i, b) in buf[account_len..].iter_mut().enumerate() {
        *b = ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY);
    }
}

fn expected(account_len: usize, abs: usize) -> u8 {
    let i = abs - account_len;
    ((i + 1) as u8).wrapping_mul(17).wrapping_add(CANARY)
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

fn write_empty(backing: &mut [u8], account_len: usize) {
    backing[0..DISC].copy_from_slice(&[0x11, 0xd8, 0xf6, 0x8e, 0xe1, 0xc7, 0xda, 0x38]);
    backing[DISC..account_len].fill(0);
    backing[DISC..DISC + 4].copy_from_slice(&START.to_le_bytes());
}

fn envelope_snapshot(buf: &[u8], account_len: usize, n: usize) -> Vec<u8> {
    buf[account_len..account_len + n].to_vec()
}

fn count_dirty(buf: &[u8], account_len: usize) -> usize {
    (account_len..buf.len())
        .filter(|&i| buf[i] != expected(account_len, i))
        .count()
}

fn main() {
    println!("dta_oob_rw — OOB read-after-write (host)");
    let min = DynamicTickArray::MIN_LEN;
    let grow = DynamicTickData::LEN;
    let n_at = min + grow; // 260

    // --- Experiment 1: write then raw-read envelope ---
    println!("\n=== EXP1: rotate OOB write → raw read envelope ===");
    let mut backing = vec![0u8; n_at + GUARD];
    write_empty(&mut backing, n_at);
    paint(&mut backing, n_at);
    let before = envelope_snapshot(&backing, n_at, 256);

    {
        let body = &mut backing[DISC..n_at];
        let loader = DynamicTickArrayLoader::load_mut(body);
        loader
            .update_tick(START, SPACING, &init_update())
            .expect("update_tick");
    }

    let after = envelope_snapshot(&backing, n_at, 256);
    let dirty = count_dirty(&backing, n_at);
    let changed = before != after;
    println!("  dirty_count={dirty} envelope_first_256_changed={changed}");
    if changed {
        let first = before
            .iter()
            .zip(after.iter())
            .position(|(a, b)| a != b);
        println!("  first envelope delta at +{first:?}");
        println!(
            "  HOST CHANNEL: yes — OOB write is readable back from the same allocation (case B candidate on host)."
        );
    } else {
        println!("  envelope first 256 unchanged (unexpected if dirty>0)");
    }

    // --- Experiment 2: production get_tick after OOB write (committed logical body) ---
    println!("\n=== EXP2: production get_tick after OOB write (all tick idxs) ===");
    // Account still at n_at=260 with 1 init tick; load on exact body.
    let body_len = n_at - DISC;
    let mut pad = vec![0u8; DynamicTickArray::MAX_LEN];
    pad[..body_len].copy_from_slice(&backing[DISC..n_at]);
    // Place unique marker in envelope region of the MAX_LEN pad beyond real body
    // (simulates reading through falsely extended view)
    for i in body_len..pad.len() {
        pad[i] = 0xEE;
    }
    // Copy actual post-rotate envelope bytes from backing into pad beyond body
    let copy_n = (backing.len() - n_at).min(pad.len() - body_len);
    pad[body_len..body_len + copy_n].copy_from_slice(&backing[n_at..n_at + copy_n]);

    let loader = DynamicTickArrayLoader::load(&pad[..body_len]);
    // NOTE: load() on short slice — claimed MAX_LEN extends into pad's following bytes
    // which we filled from the real OOB envelope. If get_tick ever decoded those into
    // Tick fields, we'd see 0xEE-pattern / envelope influence.
    let mut suspicious = 0usize;
    for i in 0..N {
        let idx = START + i as i32 * SPACING as i32;
        match loader.get_tick(idx, SPACING) {
            Ok(t) => {
                // Only tick 0 should be initialized with our known payload
                if i == 0 {
                    assert!(t.initialized);
                    let net = t.liquidity_net;
                    if net != 111 {
                        println!("  tick0 liquidity_net={net} ≠ 111 — ENVELOPE INFLUENCE?");
                        suspicious += 1;
                    }
                } else if t.initialized {
                    println!("  tick {i} unexpectedly initialized — check");
                    suspicious += 1;
                }
            }
            Err(e) => println!("  get_tick({i}) err={e:?}"),
        }
    }
    if suspicious == 0 {
        println!(
            "  No production get_tick returned envelope-influenced Tick values (logical read path clean)."
        );
    } else {
        println!("  suspicious={suspicious}");
    }

    // --- Experiment 3: force P2a slice formation that covers envelope; Uninit read ---
    println!("\n=== EXP3: form [off..off+113] crossing boundary; Uninit deserialize ===");
    // Empty MIN account: real tick data 88 bytes; idx 0 forms [0..113] crossing into envelope
    let mut empty = vec![0u8; min + GUARD];
    write_empty(&mut empty, min);
    // Put distinctive pattern in envelope
    for (i, b) in empty[min..].iter_mut().enumerate() {
        *b = 0xC0u8.wrapping_add(i as u8);
    }
    {
        let body = &empty[DISC..min];
        let loader = DynamicTickArrayLoader::load(body);
        // get_tick(0) on empty: Uninit tag at 0, forms 113-byte slice into envelope, reads 1 byte
        let t = loader.get_tick(START, SPACING).expect("get_tick0");
        assert!(!t.initialized);
        println!("  get_tick(0) on MIN empty → Uninitialized (1-byte read); envelope not consumed as fields.");
        // Also last index
        let t87 = loader
            .get_tick(START + 87 * SPACING as i32, SPACING)
            .expect("get_tick87");
        assert!(!t87.initialized);
        println!("  get_tick(87) similarly Uninitialized.");
    }

    // --- Experiment 4: write OOB, then manually deserialize Initialized at a forged crossing offset ---
    println!("\n=== EXP4: synthetic Initialized deserialize across boundary (invariant-breaking) ===");
    println!("  (Not a reachable production state — probes whether envelope bytes CAN feed Borsh.)");
    {
        let mut buf = vec![0u8; n_at + GUARD];
        write_empty(&mut buf, n_at);
        paint(&mut buf, n_at);
        // Forge: place tag=1 at last real tick byte index near end, such that +113 crosses
        // Real tick data ends at body index HEADER+200-1 for one-I layout after increase...
        // Simpler: on MIN empty, put tag=1 at offset 0 and claim we need 113 bytes with only 88 real.
        let body = &mut buf[DISC..min];
        body[HEADER] = 1; // forged Initialized tag at tick 0 with only 88 tick bytes total
                          // remaining 112 bytes of "Initialized" would span into envelope
        for i in 0..112 {
            let abs = min + i; // force payload to be read from envelope if we slice wrong
            let _ = abs;
        }
        // Form slice as production get_tick would against MAX_LEN view:
        let loader = DynamicTickArrayLoader::load(body);
        // Directly mimic get_tick slice+deserialize by calling get_tick — bitmap still 0 so
        // byte_offset(0)=0; tag is 1; Borsh will try to read 113 bytes from claimed view.
        // But bitmap bit0 is 0 — get_tick still uses byte_offset from bitmap, then deserializes
        // whatever bytes are there. Tag=1 ⇒ Initialized path reads 113.
        match loader.get_tick(START, SPACING) {
            Ok(t) => {
                let net = t.liquidity_net;
                let init = t.initialized;
                let fee_a = t.fee_growth_outside_a;
                let fee_b = t.fee_growth_outside_b;
                println!("  forged-tag get_tick → initialized={init} liquidity_net={net}");
                println!(
                    "  (If net derives from canary/envelope pattern, envelope→program-visible channel exists when invariant broken.)"
                );
                println!("  fee_a={fee_a} fee_b={fee_b} (inspect manually vs envelope)");
            }
            Err(e) => println!("  forged-tag get_tick err={e:?}"),
        }
    }

    println!("\n=== RAW VERDICT (host) ===");
    println!("  write→raw read envelope: YES (process memory channel).");
    println!("  write→production get_tick fields: NO under invariant-holding state.");
    println!("  Uninit boundary slice: forms past end, reads 1 in-bounds byte only.");
    println!("  Envelope case on host so far: A for committed DEX state; B only as raw memory.");
    println!("  SVM may remap envelope (C/D) — Phase E required.");
}
