#![deny(suspicious_leading_zero)]

fn main() {
    let _ = 0111; //~ ERROR [suspicious_leading_zero]
    let _ = 0007; //~ ERROR [suspicious_leading_zero]
    let _ = 0008;
}
