use crate::abi::call::{ArgAbi, FnAbi, Reg, RegKind};
use crate::abi::{Abi, HasDataLayout, Size, TyAbiInterface, TyAndLayout};
use crate::spec::{HasS390xVector, HasTargetSpec};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ABI {
    NoVector, // no-vector ABI, i.e., compiling for a pre-z13 machine or using -C target-feature=-vector
    Vector, // vector ABI, i.e., compiling for a z13 or later machine or using -C target-feature=+vector
}
use ABI::*;

fn contains_vector<'a, Ty, C>(cx: &C, layout: TyAndLayout<'a, Ty>, expected_size: Size) -> bool
where
    Ty: TyAbiInterface<'a, C> + Copy,
{
    match layout.abi {
        Abi::Uninhabited | Abi::Scalar(_) | Abi::ScalarPair(..) => false,
        Abi::Vector { .. } => layout.size == expected_size,
        Abi::Aggregate { .. } => {
            for i in 0..layout.fields.count() {
                if contains_vector(cx, layout.field(cx, i), expected_size) {
                    return true;
                }
            }
            false
        }
    }
}

fn classify_ret<Ty>(ret: &mut ArgAbi<'_, Ty>, abi: ABI) {
    let size = ret.layout.size;
    if abi == Vector && size.bits() <= 128 && matches!(ret.layout.abi, Abi::Vector { .. }) {
        ret.cast_to(Reg { kind: RegKind::Vector, size }); // FIXME: this cast is unneeded?
        return;
    }
    if !ret.layout.is_aggregate() && size.bits() <= 64 {
        ret.extend_integer_width_to(64);
        return;
    }
    ret.make_indirect();
}

fn classify_arg<'a, Ty, C>(cx: &C, arg: &mut ArgAbi<'a, Ty>, abi: ABI)
where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout + HasTargetSpec,
{
    if !arg.layout.is_sized() {
        // Not touching this...
        return;
    }
    if arg.is_ignore() {
        // s390x-unknown-linux-{gnu,musl,uclibc} doesn't ignore ZSTs.
        if cx.target_spec().os == "linux"
            && matches!(&*cx.target_spec().env, "gnu" | "musl" | "uclibc")
            && arg.layout.is_zst()
        {
            arg.make_indirect_from_ignore();
        }
        return;
    }

    let size = arg.layout.size;
    if abi == Vector && size.bits() <= 128 && contains_vector(cx, arg.layout, size) {
        arg.cast_to(Reg { kind: RegKind::Vector, size });
        return;
    }
    if !arg.layout.is_aggregate() && size.bits() <= 64 {
        arg.extend_integer_width_to(64);
        return;
    }

    if arg.layout.is_single_fp_element(cx) {
        match size.bytes() {
            4 => arg.cast_to(Reg::f32()),
            8 => arg.cast_to(Reg::f64()),
            _ => arg.make_indirect(),
        }
    } else {
        match size.bytes() {
            1 => arg.cast_to(Reg::i8()),
            2 => arg.cast_to(Reg::i16()),
            4 => arg.cast_to(Reg::i32()),
            8 => arg.cast_to(Reg::i64()),
            _ => arg.make_indirect(),
        }
    }
}

pub(crate) fn compute_abi_info<'a, Ty, C>(cx: &C, fn_abi: &mut FnAbi<'a, Ty>)
where
    Ty: TyAbiInterface<'a, C> + Copy,
    C: HasDataLayout + HasTargetSpec + HasS390xVector,
{
    let abi = if cx.has_s390x_vector() { Vector } else { NoVector };

    if !fn_abi.ret.is_ignore() {
        classify_ret(&mut fn_abi.ret, abi);
    }

    for arg in fn_abi.args.iter_mut() {
        classify_arg(cx, arg, abi);
    }
}
