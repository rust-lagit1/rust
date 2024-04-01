use crate::spec::{base, Cc, LinkerFlavor, Lld, StackProbeType, Target, TargetMetadata};

pub fn target() -> Target {
    let mut base = base::managarm_mlibc::opts();
    base.cpu = "x86-64".to_string().into();
    base.max_atomic_width = Some(64);
    base.add_pre_link_args(LinkerFlavor::Gnu(Cc::Yes, Lld::No), &["-m64"]);
    base.stack_probes = StackProbeType::Inline;

    Target {
        llvm_target: "x86_64-unknown-managarm-mlibc".to_string().into(),
        pointer_width: 64,
        data_layout:
            "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
                .to_string()
                .into(),
        arch: "x86_64".to_string().into(),
        options: base,
        metadata: TargetMetadata { std: Some(false), tier: Some(3), ..Default::default() },
    }
}
