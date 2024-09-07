//@ compile-flags: -O
//@ min-llvm-version: 19

#![crate_type = "lib"]

pub enum State {
    A([u8; 753]),
    B([u8; 753]),
}

// CHECK-LABEL: @update
#[no_mangle]
pub unsafe fn update(s: *mut State) {
    // CHECK-NEXT: start:
    // CHECK-NEXT: [[D:%.*]] = load i8, ptr %s
    // CHECK-NEXT: [[D2:%.*]] = and i8 [[D]], 1
    // CHECK-NEXT: [[A:%.*]] = icmp eq i8 [[D2]], 0
    // CHECK-NEXT: tail call void @llvm.assume(i1 [[A]])
    // CHECK-NEXT: store i8 1, ptr %s
    // CHECK-NEXT: ret
    let State::A(v) = s.read() else { std::hint::unreachable_unchecked() };
    s.write(State::B(v));
}
