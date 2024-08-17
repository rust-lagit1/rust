#![feature(stmt_expr_attributes)]

#[instruction_set(arm::a32)]
type ValidA = ();

#[instruction_set(arm::t32)]
type ValidB = ();

#[instruction_set(asdfasdf)] //~ ERROR `[instruction_set]` attribute argument should be valid
type InvalidA = ();

#[instruction_set(asdfasdf)] //~ ERROR `[instruction_set]` attribute argument should be valid
mod InvalidB {}

#[instruction_set(asdfasdf)] //~ ERROR `[instruction_set]` attribute argument should be valid
struct InvalidC;

#[instruction_set(asdfasdf)] //~ ERROR `[instruction_set]` attribute argument should be valid
impl InvalidC {}

fn main() {}
