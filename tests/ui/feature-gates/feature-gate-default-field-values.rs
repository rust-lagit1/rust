#![feature(generic_const_exprs)]
#![allow(unused_variables, dead_code, incomplete_features)]

pub struct S;

#[derive(Default)]
pub struct Foo {
    pub bar: S = S, //~ ERROR default values on `struct` fields aren't supported
    pub baz: i32 = 42 + 3, //~ ERROR default values on `struct` fields aren't supported
}

#[derive(Default)]
pub enum Bar {
    #[default]
    Foo { //~ ERROR the `#[default]` attribute may only be used on unit enum variants
        bar: S = S, //~ ERROR default values on `struct` fields aren't supported
        baz: i32 = 42 + 3, //~ ERROR default values on `struct` fields aren't supported
    }
}

#[derive(Default)]
pub struct Qux<A, const C: i32> {
    bar: S = Qux::<A, C>::S, //~ ERROR default values on `struct` fields aren't supported
    baz: i32 = foo(), //~ ERROR default values on `struct` fields aren't supported
    bat: i32 = <Qux<A, C> as T>::K, //~ ERROR default values on `struct` fields aren't supported
    bay: i32 = C, //~ ERROR default values on `struct` fields aren't supported
    bak: Vec<A> = Vec::new(), //~ ERROR default values on `struct` fields aren't supported
}

impl<A, const C: i32> Qux<A, C> {
    const S: S = S;
}

trait T {
    const K: i32;
}

impl<A, const C: i32> T for Qux<A, C> {
    const K: i32 = 2;
}

const fn foo() -> i32 {
    42
}

fn main () {
    let x = Foo { .. }; //~ ERROR base expression required after `..`
    let y = Foo::default();
    let z = Foo { baz: 1, .. }; //~ ERROR base expression required after `..`

    assert_eq!(45, x.baz);
    assert_eq!(45, y.baz);
    assert_eq!(1, z.baz);

    let x = Bar::Foo { .. }; //~ ERROR base expression required after `..`
    //~^ ERROR functional record update syntax requires a struct
    let y = Bar::default();
    let z = Bar::Foo { baz: 1, .. }; //~ ERROR base expression required after `..`
    //~^ ERROR functional record update syntax requires a struct

    assert!(matches!(Bar::Foo { bar: S, baz: 45 }, x));
    assert!(matches!(Bar::Foo { bar: S, baz: 45 }, y));
    assert!(matches!(Bar::Foo { bar: S, baz: 1 }, z));

    let x = Qux::<i32, 4> { .. }; //~ ERROR base expression required after `..`
    assert!(matches!(Qux::<i32, 4> { bar: S, baz: 42, bat: 2, bay: 4, .. }, x));
    //~^ ERROR base expression required after `..`
    assert!(x.bak.is_empty());
}
