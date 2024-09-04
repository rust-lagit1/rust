//@ compile-flags: -Copt-level=3

#![feature(optimize_attribute)]

#[optimize(none)]
pub fn add_noopt() -> i32 {
    // CHECK-LABEL: fn add_noopt(
    // CHECK: _0 = Add(const 1_i32, const 2_i32);
    1 + 2
}

pub fn add_normal() -> i32 {
    // CHECK-LABEL: fn add_normal(
    // CHECK: _0 = const 3_i32;
    1 + 2
}

fn main() {}
