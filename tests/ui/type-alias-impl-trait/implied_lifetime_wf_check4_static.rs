#![feature(type_alias_impl_trait)]

pub type Ty<A> = impl Sized + 'static;
//~^ ERROR: the parameter type `A` may not live long enough
#[defines(Ty)]
fn defining<A: 'static>(s: A) -> Ty<A> {
    s
}
pub fn assert_static<A: 'static>() {}

fn test<A>()
where
    Ty<A>: 'static,
{
    assert_static::<A>()
    //~^ ERROR: the parameter type `A` may not live long enough
}

fn main() {}
