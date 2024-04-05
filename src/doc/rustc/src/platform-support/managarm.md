# `*-unknown-managarm-mlibc`

**Tier: 3**

## Target Maintainers

- [@no92](https://github.com/no92)
- [@64](https://github.com/64)
- [@Dennisbonke](https://github.com/Dennisbonke)

## Requirements

This target is cross-compiled. There is currently no support for `std` yet. It generates binaries in the ELF format.

## Building the target

For now, building a patched LLVM with [our patches located here](https://github.com/managarm/bootstrap-managarm/tree/master/patches/llvm) is necessary.

Once that is done, set up your `config.toml` like this:

```toml
change-id = 102579

[llvm]
targets = "X86"
download-ci-llvm = false

[build]
target = ["x86_64-unknown-managarm-mlibc", "x86_64-unknown-linux-gnu"]

[target.x86_64-unknown-linux-gnu]
llvm-config = "/path/to/your/llvm/bin/llvm-config"

[target.x86_64-unknown-managarm-mlibc]
llvm-config = "/path/to/your/llvm/bin/llvm-config"
```

## Building Rust programs

Build a `x86_64-managarm-gcc` using our [gcc fork](https://github.com/managarm/gcc).

```toml
[build]
rustc = "/path/to/the/rust-prefix/bin/rustc"
target = "x86_64-unknown-managarm-mlibc"

[target.x86_64-unknown-managarm-mlibc]
linker = "/path/to/the/managarm-gcc/bin/x86_64-managarm-gcc"
```

## Testing

This target does not support running the Rust testsuite yet.
