//@ revisions: no_fails catch_fail no_checking
//@ [no_fails]    run-pass
//@ [catch_fail]  run-fail
//@ [no_checking] run-pass

//@[no_fails]    compile-flags: -Zcontract-checking=dynamic
//@[catch_fail]  compile-flags: -Zcontract-checking=dynamic
//@[no_checking] compile-flags: -Zcontract-checking=none

#![allow(internal_features)]
#![feature(rustc_attrs, rustc_contracts, core_intrinsics)]

#[contracts::requires(x > 0)]
fn foo(x: i32) {
    let _ = x + 2;
}

fn main() {
    foo(10);

    #[cfg(not(no_fails))]
    foo(-10);
}
