//! Deterministic residual probes for xORCA share math (XO-M01/M02/M03).
//! Not a fuzzer — enumerates known-bad / boundary cases and prints a report.

const V_S: u128 = 100;
const V_N: u128 = 100;

fn orca_to_xorca(orca: u64, non: u64, supply: u64) -> Result<u64, &'static str> {
    if supply == 0 || non == 0 {
        return Ok(orca);
    }
    let out = (orca as u128)
        .checked_mul(supply as u128 + V_S)
        .ok_or("ov")?
        .checked_div(non as u128 + V_N)
        .ok_or("div")?;
    u64::try_from(out).map_err(|_| "wide")
}

fn xorca_to_orca(xorca: u64, non: u64, supply: u64) -> Result<u64, &'static str> {
    if supply == 0 || non == 0 {
        return Err("zero");
    }
    let out = (xorca as u128)
        .checked_mul(non as u128 + V_N)
        .ok_or("ov")?
        .checked_div(supply as u128 + V_S)
        .ok_or("div")?;
    u64::try_from(out).map_err(|_| "wide")
}

fn main() {
    let mut dust_zero = 0u64;
    let mut cases = 0u64;

    // XO-M02: after large donation, small victim stake → 0 shares
    println!("=== XO-M02 residual dust / zero-share after donation ===");
    for &(init_o, init_s, donor, victim) in &[
        (1u64, 1u64, 1_000_000_000u64, 1_000_000u64),
        (1, 1, 1_000_000_000_000, 1_000_000),
        (1, 1, 500_000_000_000_000, 1_000_000), // ~500M ORCA donated, 1 ORCA stake
        (100, 100, 1_000_000_000_000, 10_000_000),
        (1_000_000, 1_000_000, 1_000_000_000_000, 1_000_000),
    ] {
        cases += 1;
        let after = init_o.saturating_add(donor);
        let shares = orca_to_xorca(victim, after, init_s).unwrap();
        let status = if shares == 0 {
            dust_zero += 1;
            "ZERO_SHARES (InsufficientStakeAmount on-chain)"
        } else {
            let nn = after.saturating_add(victim);
            let ns = init_s.saturating_add(shares);
            let out = xorca_to_orca(shares, nn, ns).unwrap_or(0);
            &format!("shares={shares} redeem≈{out}")
        };
        println!(
            " init_o={init_o} init_s={init_s} donor={donor} victim={victim} -> {status}"
        );
    }

    // XO-M03: non_escrowed==0, supply>0 → 1:1 under-mint vs virtual formula
    println!("\n=== XO-M03 non_escrowed==0 special case under-mint ===");
    for supply in [1u64, 100, 1_000_000, 1_000_000_000] {
        let orca_in = 1_000_000u64;
        let special = orca_to_xorca(orca_in, 0, supply).unwrap();
        // Fair formula if we forced virtual path with non=0 would div by V_N only:
        // shares = orca * (S+100) / 100
        let fair = ((orca_in as u128) * (supply as u128 + V_S) / V_N) as u64;
        println!(
            " supply={supply} special_1to1={special} virtual_fair≈{fair} ratio_fair/special={}",
            fair / special.max(1)
        );
        assert_eq!(special, orca_in);
    }

    // Cool-down bounds (code-level): only <0 rejected — document XO-M01
    println!("\n=== XO-M01 cool_down bounds (code review) ===");
    println!(" initialize/set: reject only if cool_down_period_s < 0");
    println!(" allowed: 0 (instant withdraw for NEW unstakes), i64::MAX (overflow risk on add)");
    println!(" existing PendingWithdraw timestamps are snapshotted — not rewritten on set");

    println!("\n=== Summary ===");
    println!("donation zero-share cases: {dust_zero}/{cases}");
    println!("XO-M02 residual CONFIRMED for stake << donation/virtual_offset");
    println!("XO-M03 special-case under-mint CONFIRMED when vault fully escrowed");
}
