//@ assembly-output: emit-asm
//@ revisions: riscv64 riscv64-zbb
//@ compile-flags: -C opt-level=3
//@ [riscv64] only-riscv64
//@ [riscv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [riscv64] needs-llvm-components: riscv
//@ [riscv64-zbb] only-riscv64
//@ [riscv64-zbb] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [riscv64-zbb] compile-flags: -C target-feature=+zbb
//@ [riscv64-zbb] needs-llvm-components: riscv

#![crate_type = "lib"]

#[no_mangle]
// CHECK-LABEL: issue_114508_u32:
pub fn issue_114508_u32(a: u32, b: u32) -> u32 {
    // CHECK-NEXT:       .cfi_startproc

    // riscv64-NEXT:     bltu a1, a0, .[[RET:.+]]
    // riscv64-NEXT:     mv a0, a1
    // riscv64-NEXT: .[[RET]]:

    // riscv64-zbb-NEXT: maxu a0, a0, a1

    // CHECK-NEXT:       ret
    u32::max(a, b)
}

#[no_mangle]
// CHECK-LABEL: issue_114508_i32:
pub fn issue_114508_i32(a: i32, b: i32) -> i32 {
    // CHECK-NEXT:       .cfi_startproc

    // riscv64-NEXT:     blt a1, a0, .[[RET:.+]]
    // riscv64-NEXT:     mv a0, a1
    // riscv64-NEXT: .[[RET]]:

    // riscv64-zbb-NEXT: max a0, a0, a1

    // CHECK-NEXT:       ret
    i32::max(a, b)
}
