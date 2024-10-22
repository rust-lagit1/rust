//@ revisions: all strong basic none
//@ ignore-nvptx64 stack protector not supported
//@ [all] compile-flags: -C stack-protector=all
//@ [strong] compile-flags: -C stack-protector=strong -Z unstable-options
//@ [basic] compile-flags: -C stack-protector=basic -Z unstable-options

#![crate_type = "lib"]

#[no_mangle]
pub fn foo() {
    // CHECK: @foo() unnamed_addr #0

    // all-NOT: attributes #0 = { {{.*}}sspstrong {{.*}} }
    // all-NOT: attributes #0 = { {{.*}}ssp {{.*}} }
    // all: attributes #0 = { {{.*}}sspreq {{.*}} }
    // all-NOT: attributes #0 = { {{.*}}sspstrong {{.*}} }
    // all-NOT: attributes #0 = { {{.*}}ssp {{.*}} }

    // strong-NOT: attributes #0 = { {{.*}}sspreq {{.*}} }
    // strong-NOT: attributes #0 = { {{.*}}ssp {{.*}} }
    // strong: attributes #0 = { {{.*}}sspstrong {{.*}} }
    // strong-NOT: attributes #0 = { {{.*}}sspreq {{.*}} }
    // strong-NOT: attributes #0 = { {{.*}}ssp {{.*}} }

    // basic-NOT: attributes #0 = { {{.*}}sspreq {{.*}} }
    // basic-NOT: attributes #0 = { {{.*}}sspstrong {{.*}} }
    // basic: attributes #0 = { {{.*}}ssp {{.*}} }
    // basic-NOT: attributes #0 = { {{.*}}sspreq {{.*}} }
    // basic-NOT: attributes #0 = { {{.*}}sspstrong {{.*}} }

    // none-NOT: attributes #0 = { {{.*}}sspreq {{.*}} }
    // none-NOT: attributes #0 = { {{.*}}sspstrong {{.*}} }
    // none-NOT: attributes #0 = { {{.*}}ssp {{.*}} }
}
