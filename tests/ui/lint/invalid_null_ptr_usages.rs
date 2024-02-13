// check-fail
// run-rustfix

use std::ptr;
use std::mem;

unsafe fn null_ptr() {
    ptr::write(
    //~^ ERROR calling this function with a null pointer is undefined behavior
        ptr::null_mut() as *mut u32,
        mem::transmute::<[u8; 4], _>([0, 0, 0, 255]),
    );

    let _: &[usize] = std::slice::from_raw_parts(ptr::null(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _: &[usize] = std::slice::from_raw_parts(ptr::null_mut(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _: &[usize] = std::slice::from_raw_parts(0 as *mut _, 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _: &[usize] = std::slice::from_raw_parts(mem::transmute(0usize), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    let _: &[usize] = std::slice::from_raw_parts_mut(ptr::null_mut(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::copy::<usize>(ptr::null(), ptr::NonNull::dangling().as_ptr(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    ptr::copy::<usize>(ptr::NonNull::dangling().as_ptr(), ptr::null_mut(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::copy_nonoverlapping::<usize>(ptr::null(), ptr::NonNull::dangling().as_ptr(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    ptr::copy_nonoverlapping::<usize>(
    //~^ ERROR calling this function with a null pointer is undefined behavior
        ptr::NonNull::dangling().as_ptr(),
        ptr::null_mut(),
        0
    );

    struct A; // zero sized struct
    assert_eq!(std::mem::size_of::<A>(), 0);

    let _a: A = ptr::read(ptr::null());
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _a: A = ptr::read(ptr::null_mut());
    //~^ ERROR calling this function with a null pointer is undefined behavior

    let _a: A = ptr::read_unaligned(ptr::null());
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _a: A = ptr::read_unaligned(ptr::null_mut());
    //~^ ERROR calling this function with a null pointer is undefined behavior

    let _a: A = ptr::read_volatile(ptr::null());
    //~^ ERROR calling this function with a null pointer is undefined behavior
    let _a: A = ptr::read_volatile(ptr::null_mut());
    //~^ ERROR calling this function with a null pointer is undefined behavior

    let _a: A = ptr::replace(ptr::null_mut(), A);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::swap::<A>(ptr::null_mut(), &mut A);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    ptr::swap::<A>(&mut A, ptr::null_mut());
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::swap_nonoverlapping::<A>(ptr::null_mut(), &mut A, 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
    ptr::swap_nonoverlapping::<A>(&mut A, ptr::null_mut(), 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::write(ptr::null_mut(), A);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::write_unaligned(ptr::null_mut(), A);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::write_volatile(ptr::null_mut(), A);
    //~^ ERROR calling this function with a null pointer is undefined behavior

    ptr::write_bytes::<usize>(ptr::null_mut(), 42, 0);
    //~^ ERROR calling this function with a null pointer is undefined behavior
}

fn main() {
    unsafe { null_ptr() };
}
