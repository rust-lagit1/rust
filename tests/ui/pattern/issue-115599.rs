//! Test that a type with derived (Partial)Eq, and thus Structural(Partial)Eq
//! that is a newtype over a type without Structural(Partial)Eq correctly errors.

// `String` in the stdlib no longer derives these traits, so make our own here.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct String(Vec<u8>);

const CONST_STRING: String = String(Vec::new());

fn main() {
    let empty_str = String(Vec::from(&[]));
    if let CONST_STRING = empty_str {}
    //~^ ERROR to use a constant of type `Vec<u8>` in a pattern, `Vec<u8>` must be annotated with `#[derive(PartialEq)]`
}
