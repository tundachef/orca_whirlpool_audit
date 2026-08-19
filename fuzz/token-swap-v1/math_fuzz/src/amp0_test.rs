// standalone test binary for amp=0 behavior documentation
fn main() {
    // leverage = amp * 2; amp=0 => leverage=0
    // compute_d with leverage 0: in SPL, d_val.checked_div(leverage) fails => None
    println!("amp=0 => swap_without_fees returns None (fail closed)");
    println!("deposit/withdraw proportional helpers ignore amp in StableCurve impl");
    println!("Conclusion: amp=0 pool can be created+LP'd but swaps fail — grief/honeypot");
}
