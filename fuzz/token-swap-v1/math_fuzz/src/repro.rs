use arbitrary::{Arbitrary, Unstructured};
const N_COINS: u8 = 2;
#[derive(Debug, Arbitrary, Clone)]
struct Fees { trade_num: u16, trade_den: u16, owner_num: u16, owner_den: u16 }
fn calc_fee(amount: u128, num: u128, den: u128) -> Option<u128> {
    if num == 0 || amount == 0 { return Some(0); }
    if den == 0 { return None; }
    let fee = amount.checked_mul(num)?.checked_div(den)?;
    Some(if fee == 0 { 1 } else { fee })
}
fn cp_swap(x: u128, y: u128, dx: u128) -> Option<u128> {
    if x == 0 || y == 0 || dx == 0 { return None; }
    y.checked_mul(dx)?.checked_div(x.checked_add(dx)?)
}
fn stable_get_d(amount_a: u128, amount_b: u128, amp: u64) -> Option<u128> {
    let sum = amount_a.checked_add(amount_b)?;
    if sum == 0 { return Some(0); }
    let n = N_COINS as u128;
    let amp_n = (amp as u128).checked_mul(n)?;
    let mut d = sum;
    for _ in 0..32 {
        let mut d_p = d;
        d_p = d_p.checked_mul(d)?.checked_div(amount_a.checked_mul(n)?)?;
        d_p = d_p.checked_mul(d)?.checked_div(amount_b.checked_mul(n)?)?;
        let d_prev = d;
        let numerator = amp_n.checked_mul(sum)?.checked_add(d_p.checked_mul(n)?)?.checked_mul(d)?;
        let denominator = amp_n.checked_sub(1)?.checked_mul(d)?.checked_add(d_p.checked_mul(n.checked_add(1)?)?)?;
        if denominator == 0 { return None; }
        d = numerator.checked_div(denominator)?;
        if d.abs_diff(d_prev) <= 1 { return Some(d); }
    }
    Some(d)
}
#[derive(Debug, Arbitrary)]
enum Action {
    CpSwap { x: u64, y: u64, dx: u64 },
    StableD { a: u64, b: u64, amp: u16 },
    Fee { amount: u64, fees: Fees },
}
fn main() {
    let data = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let mut u = Unstructured::new(&data);
    let action = Action::arbitrary(&mut u).expect("arb");
    eprintln!("ACTION {action:?}");
    match action {
        Action::CpSwap { x, y, dx } => {
            let x = (x as u128) % (1u128 << 80) + 1;
            let y = (y as u128) % (1u128 << 80) + 1;
            let dx = (dx as u128) % (1u128 << 80);
            if let Some(dy) = cp_swap(x, y, dx) {
                assert!(dy <= y, "CP dy>y");
                let xp = x.saturating_add(dx);
                let yp = y.saturating_sub(dy);
                if let (Some(k0), Some(k1)) = (x.checked_mul(y), xp.checked_mul(yp)) {
                    assert!(k1 >= k0, "CP k decreased k0={k0} k1={k1}");
                }
            }
        }
        Action::StableD { a, b, amp } => {
            let a = (a as u128) % (1u128 << 60) + 1;
            let b = (b as u128) % (1u128 << 60) + 1;
            let amp = (amp as u64) % 10_000;
            let d = stable_get_d(a, b, amp);
            eprintln!("stable a={a} b={b} amp={amp} d={d:?}");
            if amp != 0 {
                if let Some(d) = d {
                    assert!(d > 0, "D==0");
                    let bound = a.saturating_add(b).saturating_mul(4).saturating_add(1_000_000);
                    assert!(d < bound, "D exploded d={d} bound={bound}");
                }
            }
        }
        Action::Fee { amount, fees } => {
            let amount = amount as u128;
            let trade_ok = (fees.trade_num == 0 && fees.trade_den == 0) || (fees.trade_den != 0 && fees.trade_num < fees.trade_den);
            let owner_ok = (fees.owner_num == 0 && fees.owner_den == 0) || (fees.owner_den != 0 && fees.owner_num < fees.owner_den);
            let r1 = calc_fee(amount, fees.trade_num as u128, fees.trade_den as u128);
            let r2 = calc_fee(amount, fees.owner_num as u128, fees.owner_den as u128);
            if fees.trade_den == 0 && fees.trade_num != 0 { assert!(r1.is_none()); }
            if trade_ok { if let Some(f) = r1 { assert!(f <= amount || amount == 0, "trade"); } }
            if owner_ok { if let Some(f) = r2 { assert!(f <= amount || amount == 0, "owner"); } }
        }
    }
    eprintln!("OK");
}
