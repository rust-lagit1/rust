//@check-fail
struct Foo{
    bar: u32,
}

impl Default for Foo {
    fn default() -> Self {
        Self {
            ..Default::default()
            //~^ ERROR: recursive default impl
        }
    }
}
