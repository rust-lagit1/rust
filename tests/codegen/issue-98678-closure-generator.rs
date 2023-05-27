// This test verifies the accuracy of emitted file and line debuginfo metadata for closures and
// generators.
//
// compile-flags: -C debuginfo=2
#![crate_type = "lib"]
#![feature(generators)]

// ignore-tidy-linelength

// NONMSVC: ![[#FILE:]] = !DIFile({{.*}}filename:{{.*}}/codegen/issue-98678-closure-generator.rs{{".*}})
// MSVC: ![[#FILE:]] = !DIFile({{.*}}filename:{{.*}}\\codegen\\issue-98678-closure-generator.rs{{".*}})

pub fn foo() {
    // NONMSVC: !DICompositeType({{.*"[{]}}generator_env#0{{[}]".*}}file: ![[#FILE]]{{.*}}line: [[# @LINE + 2]],
    // MSVC-DAG: !DICompositeType({{.*".*foo::}}generator_env$0>{{".*}}file: ![[#FILE]]{{.*}}line: [[# @LINE + 1]],
    let generator = || yield 1;
}
